package com.aether.android.service

import android.app.Notification
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.net.VpnService
import android.os.Build
import android.os.ParcelFileDescriptor
import androidx.core.app.NotificationCompat
import com.aether.android.AetherApplication
import com.aether.android.R
import com.aether.android.model.Profile
import com.aether.android.model.SplitTunnelMode
import com.aether.android.model.VpnState
import com.aether.android.model.VpnStatus
import com.aether.android.ui.MainActivity
import hev.sockstun.TProxyService
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import java.io.BufferedReader
import java.io.File
import java.io.InputStreamReader
import java.net.InetSocketAddress
import java.net.Socket

class AetherVpnService : VpnService() {

    companion object {
        const val ACTION_CONNECT = "com.aether.android.CONNECT"
        const val ACTION_DISCONNECT = "com.aether.android.DISCONNECT"
        const val ACTION_RESCAN = "com.aether.android.RESCAN"
        const val NOTIFICATION_ID = 1819

        private val _vpnStatus = MutableStateFlow(VpnStatus())
        val vpnStatus: StateFlow<VpnStatus> = _vpnStatus.asStateFlow()

        private val DEFAULT_WARP_IDENTITY = """
device_id = "4df2f838-0671-44db-9850-fe63f2a01741"
access_token = "ebdcabbd-ff26-4bbd-a3ac-6385a5ee4ee7"
cert_pem = ""
key_pem = ""
cert_issued_at = 0
ipv4 = "172.16.0.2"
ipv6 = "2606:4700:110:8dbd:6465:9f60:43b7:3b5a"
wg_private_key = "6GNFMS4ruRl+bYlnXQOm5leYwCOXzVSBWmnS0JTGHW0="
wg_peer_public_key = "bmXOC+F1FxEMF9dyiK2H5/1SUtzH0JuVo51h2wPfgyo="
client_id = "dYix"
organization = ""
gateway_proxy = "172.16.0.1:2480"
assigned_endpoint = "162.159.192.10"
""".trimIndent()

        fun start(context: Context, profileId: String? = null) {
            val intent = Intent(context, AetherVpnService::class.java).apply {
                action = ACTION_CONNECT
                putExtra("profile_id", profileId)
            }
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                context.startForegroundService(intent)
            } else {
                context.startService(intent)
            }
        }

        fun stop(context: Context) {
            val intent = Intent(context, AetherVpnService::class.java).apply {
                action = ACTION_DISCONNECT
            }
            context.startService(intent)
        }

        fun rescan(context: Context) {
            val intent = Intent(context, AetherVpnService::class.java).apply {
                action = ACTION_RESCAN
            }
            context.startService(intent)
        }
    }

    private val serviceScope = CoroutineScope(Dispatchers.IO + SupervisorJob())
    private var vpnInterface: ParcelFileDescriptor? = null
    private var aetherProcess: Process? = null
    private var candidateMonitoringJob: Job? = null
    private var speedMonitoringJob: Job? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            ACTION_CONNECT -> {
                val profileId = intent.getStringExtra("profile_id")
                startVpn(profileId, forceFreshScan = false)
            }
            ACTION_DISCONNECT -> {
                stopVpn()
            }
            ACTION_RESCAN -> {
                val profileId = intent.getStringExtra("profile_id")
                startVpn(profileId, forceFreshScan = true)
            }
        }
        return START_STICKY
    }

    private fun startVpn(profileId: String?, forceFreshScan: Boolean) {
        val app = AetherApplication.instance
        val settings = app.settingsRepository.settings.value
        val targetId = profileId ?: settings.selectedProfileId
        val profile = app.profileRepository.profiles.value.find { it.id == targetId }
            ?: app.profileRepository.profiles.value.firstOrNull()

        if (profile == null) {
            _vpnStatus.value = VpnStatus(state = VpnState.ERROR, errorMessage = "No server profile selected")
            stopSelf()
            return
        }

        _vpnStatus.value = VpnStatus(
            state = VpnState.CONNECTING,
            connectedProfile = profile,
            huntingStatus = "Hunting for clean Cloudflare edge candidate..."
        )

        startForeground(NOTIFICATION_ID, buildNotification("Connecting...", profile.name))

        serviceScope.launch {
            try {
                // 1. Locate Native Aether Core Daemon
                val nativeLibDir = File(applicationInfo.nativeLibraryDir)
                val aetherBin = File(nativeLibDir, "libaether.so")
                if (!aetherBin.exists()) {
                    throw IllegalStateException("libaether.so missing in ${nativeLibDir.absolutePath}")
                }

                // Ensure aether.toml exists with pre-seeded WARP identity if empty
                val configFile = File(filesDir, "aether.toml")
                if (!configFile.exists() || configFile.length() < 50) {
                    configFile.writeText(DEFAULT_WARP_IDENTITY)
                }

                val lastconnFile = File(filesDir, "aether-lastconn.toml")

                // 2. Build Aether Daemon Command
                val cmd = mutableListOf(
                    aetherBin.absolutePath,
                    "--config", configFile.absolutePath,
                    "--bind", "127.0.0.1:1819",
                    "--wg",
                    "-4",
                    "--noize", "gfw"
                )

                if (forceFreshScan || !lastconnFile.exists()) {
                    cmd.add("--no-quick-reconnect")
                    cmd.add("--turbo")
                } else {
                    cmd.add("--quick-reconnect")
                }

                var activeCleanIp = "162.159.192.1"
                var activeCleanPort = 2408
                var activeCleanRtt = 45L

                val pb = ProcessBuilder(cmd)
                pb.directory(filesDir)
                pb.redirectErrorStream(true)
                val proc = pb.start()
                aetherProcess = proc

                // 3. Monitor Aether Stdout in Real-Time
                candidateMonitoringJob = launch {
                    try {
                        val reader = BufferedReader(InputStreamReader(proc.inputStream))
                        var line: String? = null
                        while (isActive && reader.readLine().also { line = it } != null) {
                            val currentLine = line ?: continue
                            val parsed = parseCandidateFromLine(currentLine)
                            if (parsed != null) {
                                val (ep, rtt) = parsed
                                val parts = ep.split(":")
                                if (parts.size == 2) {
                                    activeCleanIp = parts[0]
                                    activeCleanPort = parts[1].toIntOrNull() ?: activeCleanPort
                                }
                                activeCleanRtt = rtt
                                _vpnStatus.value = _vpnStatus.value.copy(
                                    huntingStatus = "Candidate: $ep (${rtt}ms)",
                                    bestCandidateRtt = rtt
                                )
                                updateNotification("Hunting: $ep (${rtt}ms)", profile.name)
                            } else if (currentLine.contains("tunnel validated") || currentLine.contains("socks5 server listening")) {
                                _vpnStatus.value = _vpnStatus.value.copy(
                                    huntingStatus = "Cloudflare WireGuard validated. Activating TUN..."
                                )
                            }
                        }
                    } catch (ignored: Exception) {}
                }

                // 4. Strict Readiness Verification for SOCKS5 Port 1819
                var socksReady = false
                val deadline = System.currentTimeMillis() + 40_000 // 40s budget for fresh scan
                while (System.currentTimeMillis() < deadline && isActive) {
                    if (proc.isAlive == false) {
                        val exitCode = proc.exitValue()
                        throw IllegalStateException("Aether engine terminated prematurely with exit code $exitCode")
                    }
                    if (isSocksPortReady("127.0.0.1", 1819)) {
                        socksReady = true
                        break
                    }
                    delay(300)
                }

                if (!socksReady) {
                    throw IllegalStateException("Timed out waiting for Aether SOCKS5 proxy on 127.0.0.1:1819")
                }

                // 5. Establish Android VpnService TUN Interface
                val builder = Builder()
                    .setSession("Aether")
                    .setMtu(settings.mtu)
                    .addAddress("172.19.0.1", 30)
                    .addDnsServer(settings.primaryDns)
                    .addDnsServer("8.8.8.8")
                    .addRoute("0.0.0.0", 0)

                try {
                    builder.addAddress("fc00::1", 120)
                    builder.addRoute("::", 0)
                } catch (ignored: Exception) {}

                // CRITICAL ROUTING LOOP PREVENTION:
                // Exclude this app package so Aether core routes through physical WAN
                try {
                    builder.addDisallowedApplication(packageName)
                } catch (ignored: Exception) {}

                // Split Tunneling
                if (settings.selectedPackages.isNotEmpty()) {
                    when (settings.splitTunnelMode) {
                        SplitTunnelMode.BYPASS_SELECTED -> {
                            for (pkg in settings.selectedPackages) {
                                try { builder.addDisallowedApplication(pkg) } catch (ignored: Exception) {}
                            }
                        }
                        SplitTunnelMode.ONLY_SELECTED -> {
                            for (pkg in settings.selectedPackages) {
                                try { builder.addAllowedApplication(pkg) } catch (ignored: Exception) {}
                            }
                        }
                        SplitTunnelMode.ALL_APPS -> {}
                    }
                }

                vpnInterface = builder.establish()
                    ?: throw IllegalStateException("Android VpnService.Builder.establish() returned null")

                val tunFd = vpnInterface!!.fd

                // 6. Start In-Process TProxy Engine (libhev-socks5-tunnel.so)
                val tproxyConf = File(filesDir, "tproxy.yml")
                tproxyConf.writeText(
                    """
                    tunnel:
                      name: tun0
                      mtu: ${settings.mtu}
                      ipv4: 172.19.0.1
                      ipv6: 'fc00::1'

                    socks5:
                      port: 1819
                      address: 127.0.0.1
                      udp: 'udp'

                    misc:
                      task-stack-size: 86016
                      tcp-buffer-size: 65536
                      udp-recv-buffer-size: 524288
                      log-level: warn
                    """.trimIndent()
                )

                val started = TProxyService.TProxyStartService(tproxyConf.absolutePath, tunFd)
                if (!started) {
                    throw IllegalStateException("Failed to start in-process TProxy engine on TUN fd $tunFd")
                }

                // 7. Transition to CONNECTED
                val activeConnectedProfile = profile.copy(
                    server = activeCleanIp,
                    port = activeCleanPort,
                    latencyMs = activeCleanRtt
                )

                _vpnStatus.value = _vpnStatus.value.copy(
                    state = VpnState.CONNECTED,
                    connectedProfile = activeConnectedProfile,
                    pingMs = activeCleanRtt,
                    huntingStatus = "Connected to clean edge $activeCleanIp:$activeCleanPort",
                    connectedSinceEpochMs = System.currentTimeMillis()
                )

                updateNotification("Connected — $activeCleanIp", activeConnectedProfile.name)
                startSpeedMonitoring()

            } catch (e: Exception) {
                _vpnStatus.value = VpnStatus(
                    state = VpnState.ERROR,
                    errorMessage = e.message ?: "Connection error"
                )
                stopVpn()
            }
        }
    }

    private fun isSocksPortReady(host: String, port: Int): Boolean {
        return try {
            val s = Socket()
            s.connect(InetSocketAddress(host, port), 250)
            s.close()
            true
        } catch (e: Exception) {
            false
        }
    }

    private fun parseCandidateFromLine(line: String): Pair<String, Long>? {
        val lower = line.lowercase()
        if (!lower.contains("candidate") && !lower.contains("endpoint")) return null

        val epRegex = Regex("(\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}):(\\d{2,5})")
        val epMatch = epRegex.find(line) ?: return null
        val endpoint = epMatch.value

        val rttRegex = Regex("(?:rtt|latency|ping)[=:\\s]+([\\d.]+)\\s*(ms|s|µs|us)?", RegexOption.IGNORE_CASE)
        val rttMatch = rttRegex.find(line)
        val rttMs = if (rttMatch != null) {
            val numStr = rttMatch.groupValues[1]
            val unit = rttMatch.groupValues.getOrNull(2) ?: "ms"
            val num = numStr.toDoubleOrNull() ?: 50.0
            if (unit.equals("s", ignoreCase = true)) {
                (num * 1000).toLong()
            } else if (unit.equals("us", ignoreCase = true) || unit.equals("µs", ignoreCase = true)) {
                (num / 1000).toLong().coerceAtLeast(1)
            } else {
                num.toLong()
            }
        } else {
            45L
        }
        return endpoint to rttMs
    }

    private fun startSpeedMonitoring() {
        speedMonitoringJob = serviceScope.launch {
            var lastTxBytes = 0L
            var lastRxBytes = 0L
            var lastSampleTime = System.currentTimeMillis()

            while (isActive) {
                delay(1200)
                if (_vpnStatus.value.state == VpnState.CONNECTED) {
                    val stats = TProxyService.TProxyGetStats()
                    val now = System.currentTimeMillis()
                    val deltaSec = ((now - lastSampleTime) / 1000.0).coerceAtLeast(0.1)

                    var upSpeed = 0L
                    var downSpeed = 0L

                    if (stats != null && stats.size >= 4) {
                        val currentTxBytes = stats[1]
                        val currentRxBytes = stats[3]
                        if (lastTxBytes > 0L && currentTxBytes >= lastTxBytes) {
                            upSpeed = ((currentTxBytes - lastTxBytes) / deltaSec).toLong()
                        }
                        if (lastRxBytes > 0L && currentRxBytes >= lastRxBytes) {
                            downSpeed = ((currentRxBytes - lastRxBytes) / deltaSec).toLong()
                        }
                        lastTxBytes = currentTxBytes
                        lastRxBytes = currentRxBytes
                        lastSampleTime = now
                    }

                    _vpnStatus.value = _vpnStatus.value.copy(
                        uplinkSpeedBps = upSpeed,
                        downlinkSpeedBps = downSpeed
                    )
                }
            }
        }
    }

    private fun stopVpn() {
        speedMonitoringJob?.cancel()
        candidateMonitoringJob?.cancel()

        try {
            TProxyService.TProxyStopService()
        } catch (ignored: Exception) {}

        try {
            aetherProcess?.destroy()
        } catch (ignored: Exception) {}
        aetherProcess = null

        try {
            vpnInterface?.close()
        } catch (ignored: Exception) {}
        vpnInterface = null

        _vpnStatus.value = VpnStatus(state = VpnState.DISCONNECTED)
        stopForeground(true)
        stopSelf()
    }

    private fun buildNotification(statusText: String, profileName: String): Notification {
        val openIntent = Intent(this, MainActivity::class.java)
        val pendingOpen = PendingIntent.getActivity(
            this, 0, openIntent,
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
        )

        val disconnectIntent = Intent(this, AetherVpnService::class.java).apply {
            action = ACTION_DISCONNECT
        }
        val pendingDisconnect = PendingIntent.getService(
            this, 1, disconnectIntent,
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
        )

        return NotificationCompat.Builder(this, AetherApplication.VPN_CHANNEL_ID)
            .setSmallIcon(R.drawable.ic_tile)
            .setContentTitle("Aether — $statusText")
            .setContentText(profileName)
            .setContentIntent(pendingOpen)
            .addAction(R.drawable.ic_tile, getString(R.string.action_disconnect), pendingDisconnect)
            .setOngoing(true)
            .setPriority(NotificationCompat.PRIORITY_LOW)
            .build()
    }

    private fun updateNotification(statusText: String, profileName: String) {
        val notification = buildNotification(statusText, profileName)
        val notificationManager = getSystemService(Context.NOTIFICATION_SERVICE) as android.app.NotificationManager
        notificationManager.notify(NOTIFICATION_ID, notification)
    }

    override fun onDestroy() {
        stopVpn()
        serviceScope.cancel()
        super.onDestroy()
    }
}
