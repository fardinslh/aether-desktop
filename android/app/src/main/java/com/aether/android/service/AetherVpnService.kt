package com.aether.android.service

import android.app.Notification
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.net.VpnService
import android.os.Build
import android.os.ParcelFileDescriptor
import android.util.Log
import androidx.core.app.NotificationCompat
import com.aether.android.AetherApplication
import com.aether.android.R
import com.aether.android.model.AppSettings
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
import java.io.InputStream
import java.io.InputStreamReader
import java.net.InetSocketAddress
import java.net.Socket
import java.util.Collections

class AetherVpnService : VpnService() {

    companion object {
        private const val TAG = "AetherVpnService"
        const val ACTION_CONNECT = "com.aether.android.CONNECT"
        const val ACTION_DISCONNECT = "com.aether.android.DISCONNECT"
        const val ACTION_RESCAN = "com.aether.android.RESCAN"
        const val NOTIFICATION_ID = 1819
        const val DEBUG_LOG = "aether-debug.log"
        const val MAX_AUTO_RECONNECTS = 3

        private val _vpnStatus = MutableStateFlow(VpnStatus())
        val vpnStatus: StateFlow<VpnStatus> = _vpnStatus.asStateFlow()

        /** Samsung logd silently drops app-tagged logcat lines, so all
         *  instrumentation also appends to a file readable via
         *  `adb shell run-as com.aether.android.debug`. */
        fun debugLog(context: Context, message: String) {
            try {
                val ts = java.text.SimpleDateFormat("HH:mm:ss.SSS", java.util.Locale.US)
                    .format(java.util.Date())
                java.io.File(context.filesDir, DEBUG_LOG).appendText("[$ts] $message\n")
            } catch (ignored: Throwable) {}
        }

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
    private var connectJob: Job? = null
    private var candidateMonitoringJob: Job? = null
    private var speedMonitoringJob: Job? = null
    private var healthWatchdogJob: Job? = null
    private var lastStartId: Int = -1

    /**
     * Mid-session endpoint death (GFW blocking the selected WireGuard edge)
     * must not dead-end the session: the watchdog re-hunts a fresh endpoint
     * up to MAX_AUTO_RECONNECTS times before surfacing a terminal ERROR.
     */
    private var autoReconnectAttempts = 0
    private var lastProfileId: String? = null
    private var lastProfileName: String = "Aether"
    private var userInitiatedStop = false

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        lastStartId = startId
        Log.i(TAG, "onStartCommand action=${intent?.action} startId=$startId flags=$flags")
        debugLog(this, "onStartCommand action=${intent?.action} startId=$startId flags=$flags")
        when (intent?.action) {
            ACTION_CONNECT -> {
                val profileId = intent.getStringExtra("profile_id")
                Log.i(TAG, "ACTION_CONNECT profileId=$profileId")
                userInitiatedStop = false
                startVpn(profileId, forceFreshScan = false)
            }
            ACTION_DISCONNECT -> {
                Log.i(TAG, "ACTION_DISCONNECT")
                userInitiatedStop = true
                stopVpn()
            }
            ACTION_RESCAN -> {
                val profileId = intent.getStringExtra("profile_id")
                Log.i(TAG, "ACTION_RESCAN profileId=$profileId")
                userInitiatedStop = false
                startVpn(profileId, forceFreshScan = true)
            }
            else -> {
                // START_STICKY restart with a null intent carries no connect
                // request; exit cleanly instead of idling without a foreground
                // notification (Android would kill the service anyway).
                Log.i(TAG, "onStartCommand null/unknown intent -> stopSelf($startId)")
                stopSelf(startId)
                return START_NOT_STICKY
            }
        }
        return START_STICKY
    }

    private fun startVpn(profileId: String?, forceFreshScan: Boolean) {
        val app = AetherApplication.instance
        val settings = app.settingsRepository.settings.value
        val targetId = profileId ?: settings.selectedProfileId
        val profilesSnapshot = app.profileRepository.profiles.value
        val profile = profilesSnapshot.find { it.id == targetId }
            ?: profilesSnapshot.firstOrNull()

        Log.i(
            TAG,
            "startVpn profileId=$profileId targetId=$targetId " +
                "selectedProfileId=${settings.selectedProfileId} " +
                "profiles=${profilesSnapshot.map { it.id }} resolved=${profile?.id} forceFreshScan=$forceFreshScan"
        )
        debugLog(
            this,
            "startVpn profileId=$profileId targetId=$targetId " +
                "profiles=${profilesSnapshot.map { it.id }} resolved=${profile?.id} forceFreshScan=$forceFreshScan"
        )

        if (profile == null) {
            Log.e(TAG, "startVpn ABORT: profile is null. profiles.size=${profilesSnapshot.size} snapshot=$profilesSnapshot")
            debugLog(this, "startVpn ABORT: profile null. profiles=${profilesSnapshot.size} snapshot=$profilesSnapshot")
            _vpnStatus.value = VpnStatus(state = VpnState.ERROR, errorMessage = "No server profile selected")
            // Conditional stop: a newer ACTION_CONNECT delivered after this
            // request must keep the service alive for its own attempt.
            stopSelf(lastStartId)
            return
        }

        // Re-entry guard: cancel any in-flight attempt synchronously so a
        // stale coroutine can never error-stomp or kill this fresh attempt.
        connectJob?.cancel()

        lastProfileId = profile.id
        lastProfileName = profile.name

        val huntingMessage = if (autoReconnectAttempts > 0) {
            "Connection lost — re-hunting a fresh endpoint (attempt $autoReconnectAttempts/$MAX_AUTO_RECONNECTS)..."
        } else {
            "Hunting for clean Cloudflare edge candidate..."
        }

        _vpnStatus.value = VpnStatus(
            state = VpnState.CONNECTING,
            connectedProfile = profile,
            huntingStatus = huntingMessage
        )

        // Android 14+ (API 34+) MUST specify foregroundServiceType matching the manifest
        val notification = buildNotification("Connecting...", profile.name)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
            startForeground(
                NOTIFICATION_ID,
                notification,
                ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE
            )
        } else {
            startForeground(NOTIFICATION_ID, notification)
        }

        connectJob = serviceScope.launch {
            try {
                // 0. Tear down any previous transport and free port 1819.
                //    Without this, a leaked daemon from an earlier attempt keeps
                //    port 1819 open, the readiness check below passes instantly
                //    against the OLD process, and the session is declared
                //    CONNECTED while routing into a dead tunnel.
                Log.i(TAG, "connect[gen]: teardown of previous transport")
                teardownTransport()

                val portReleaseDeadline = System.currentTimeMillis() + 3_000
                while (isSocksPortReady("127.0.0.1", 1819) &&
                    System.currentTimeMillis() < portReleaseDeadline
                ) {
                    delay(100)
                }
                if (isSocksPortReady("127.0.0.1", 1819)) {
                    throw IllegalStateException(
                        "Port 1819 is still held by another Aether process after teardown"
                    )
                }
                Log.i(TAG, "connect[gen]: port 1819 free, spawning daemon")
                debugLog(this@AetherVpnService, "connect: port 1819 free before spawn")

                // 1. Locate Native Aether Core Daemon
                val nativeLibDir = File(applicationInfo.nativeLibraryDir)
                val aetherBin = File(nativeLibDir, "libaether.so")
                if (!aetherBin.exists()) {
                    throw IllegalStateException("libaether.so missing in ${nativeLibDir.absolutePath}")
                }
                if (!aetherBin.canExecute()) {
                    aetherBin.setExecutable(true)
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
                Log.i(TAG, "connect[gen]: daemon spawned proc=$proc cmd=$cmd")
                debugLog(this@AetherVpnService, "connect: daemon spawned, waiting for port 1819")

                val recentLogs = Collections.synchronizedList(mutableListOf<String>())

                // 3. Monitor Aether Stdout in Real-Time
                candidateMonitoringJob = launch {
                    try {
                        val reader = BufferedReader(InputStreamReader(proc.inputStream, Charsets.UTF_8))
                        var line: String? = null
                        while (isActive && reader.readLine().also { line = it } != null) {
                            val currentLine = line ?: continue
                            Log.d(TAG, "[Aether] $currentLine")
                            recentLogs.add(currentLine)
                            if (recentLogs.size > 20) {
                                recentLogs.removeAt(0)
                            }

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
                    } catch (ignored: Throwable) {}
                }

                // 4. Strict Readiness Verification for SOCKS5 Port 1819.
                //    Guaranteed to probe THIS process: teardown above freed the
                //    port and rejected any foreign holder before spawn.
                var socksReady = false
                var portChecks = 0
                val deadline = System.currentTimeMillis() + 45_000 // 45s budget
                while (System.currentTimeMillis() < deadline && isActive) {
                    if (!proc.isAlive) {
                        val exitCode = proc.exitValue()
                        val tail = synchronized(recentLogs) { recentLogs.takeLast(3).joinToString(" | ") }
                        debugLog(this@AetherVpnService, "connect: daemon exited code=$exitCode after $portChecks checks. tail=$tail")
                        throw IllegalStateException("Aether core exited ($exitCode): $tail")
                    }
                    portChecks++
                    if (isSocksPortReady("127.0.0.1", 1819)) {
                        socksReady = true
                        debugLog(this@AetherVpnService, "connect: port ready after $portChecks checks")
                        break
                    }
                    delay(250)
                }

                if (!socksReady) {
                    val tail = synchronized(recentLogs) { recentLogs.takeLast(3).joinToString(" | ") }
                    debugLog(this@AetherVpnService, "connect: TIMEOUT waiting for port. daemonAlive=${proc.isAlive} tail=$tail")
                    throw IllegalStateException("Timed out waiting for Aether SOCKS5 proxy on 127.0.0.1:1819. Last log: $tail")
                }
                Log.i(TAG, "connect[gen]: SOCKS5 port ready, establishing TUN")
                debugLog(this@AetherVpnService, "connect: establishing TUN")

                // 5. Establish Android VpnService TUN Interface with MapDNS (anti-censorship)
                vpnInterface = establishVpnInterface(settings)
                    ?: throw IllegalStateException("Android VpnService.Builder.establish() returned null")

                val tunFd = vpnInterface!!.fd

                // 6. Start In-Process TProxy Engine (libhev-socks5-tunnel.so)
                val tproxyConf = File(filesDir, "tproxy.yml")
                tproxyConf.writeText(
                    """
                    tunnel:
                      name: tun0
                      mtu: ${settings.mtu}
                      ipv4: 198.18.0.1
                      icmp: 'reply'

                    socks5:
                      port: 1819
                      address: 127.0.0.1
                      udp: 'udp'

                    mapdns:
                      address: 198.18.0.2
                      port: 53
                      network: 240.0.0.0
                      netmask: 240.0.0.0
                      cache-size: 10000

                    misc:
                      task-stack-size: 86016
                      tcp-buffer-size: 65536
                      udp-recv-buffer-size: 524288
                      log-level: warn
                    """.trimIndent()
                )

                try {
                    TProxyService.TProxyStartService(tproxyConf.absolutePath, tunFd)
                } catch (t: Throwable) {
                    Log.e(TAG, "TProxyStartService native failure", t)
                    debugLog(this@AetherVpnService, "connect: TProxyStartService FAILED: ${t.message}")
                    throw IllegalStateException("Failed to start in-process TProxy engine: ${t.message}")
                }
                Log.i(TAG, "connect[gen]: TProxy engine started, CONNECTED")
                debugLog(this@AetherVpnService, "connect: TProxy started, CONNECTED ip=$activeCleanIp:$activeCleanPort rtt=$activeCleanRtt")
                autoReconnectAttempts = 0

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
                startHealthWatchdog()

            } catch (ce: CancellationException) {
                // Cancelled by a newer attempt or an explicit disconnect. The
                // caller owns the state transition; never surface this as an
                // error and never touch the replacement attempt's processes.
                debugLog(this@AetherVpnService, "connect: CANCELLED (superseded by newer attempt or disconnect)")
                throw ce
            } catch (t: Throwable) {
                Log.e(TAG, "VPN start failed", t)
                debugLog(this@AetherVpnService, "connect: FAILED ${t.javaClass.name}: ${t.message}\n${t.stackTraceToString().take(2000)}")
                if (autoReconnectAttempts > 0 && !userInitiatedStop) {
                    // A chained auto-reconnect attempt failed: continue the
                    // retry chain (or surface the terminal error) instead of
                    // dead-ending on the first failure.
                    handleTunnelFailure("Reconnect failed: ${t.message ?: t.javaClass.simpleName}")
                } else {
                    // stopVpn FIRST, then set ERROR: stopVpn resets the state to
                    // DISCONNECTED, so setting the error afterwards keeps the
                    // message visible in the UI (previously the order erased it,
                    // making failed connects look like silent no-ops).
                    stopVpn()
                    _vpnStatus.value = VpnStatus(
                        state = VpnState.ERROR,
                        errorMessage = t.message ?: "Connection error: ${t.javaClass.simpleName}"
                    )
                }
            }
        }
    }

    /**
     * Stops all background jobs, the TProxy engine, the Aether daemon, and
     * closes the TUN interface. Never touches the public state flow; callers
     * decide the final VpnStatus.
     */
    private fun teardownTransport() {
        healthWatchdogJob?.cancel()
        healthWatchdogJob = null
        speedMonitoringJob?.cancel()
        speedMonitoringJob = null
        candidateMonitoringJob?.cancel()
        candidateMonitoringJob = null

        try {
            TProxyService.TProxyStopService()
        } catch (ignored: Throwable) {}

        try {
            aetherProcess?.destroy()
        } catch (ignored: Throwable) {}
        aetherProcess = null

        try {
            vpnInterface?.close()
        } catch (ignored: Throwable) {}
        vpnInterface = null
    }

    private fun stopVpn() {
        connectJob?.cancel()
        connectJob = null

        // A user-initiated stop resets the reconnect chain entirely.
        autoReconnectAttempts = 0

        teardownTransport()

        _vpnStatus.value = VpnStatus(state = VpnState.DISCONNECTED)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.N) {
            stopForeground(STOP_FOREGROUND_REMOVE)
        } else {
            @Suppress("DEPRECATION")
            stopForeground(true)
        }
        // Conditional stop with the last seen startId: if a newer
        // ACTION_CONNECT/RESCAN arrived after this disconnect, the system
        // ignores this request and the fresh attempt's coroutine survives.
        // (Unconditional stopSelf() destroys the service even then, and the
        // resulting onDestroy() cancels serviceScope, silently killing the
        // new attempt mid-flight — a "connect does nothing" failure mode.)
        Log.i(TAG, "stopVpn complete -> stopSelf($lastStartId)")
        debugLog(this, "stopVpn complete -> stopSelf($lastStartId)")
        stopSelf(lastStartId)
    }

    /**
     * Post-connect liveness watchdog. A port-1819 listener staying open says
     * nothing about the upstream WARP tunnel: the endpoint can be DPI-blocked
     * mid-session or the daemon can exit after a network switch, leaving the
     * UI stuck on CONNECTED while all traffic blackholes. The watchdog probes
     * the full data path (SOCKS5 greeting + CONNECT to a remote domain through
     * the tunnel) and surfaces ERROR the moment the tunnel stops passing
     * traffic instead of pretending to be connected.
     */
    private fun startHealthWatchdog() {
        healthWatchdogJob = serviceScope.launch {
            var firstCheck = true
            var checks = 0
            while (isActive) {
                delay(if (firstCheck) 5_000 else 10_000)
                firstCheck = false
                if (_vpnStatus.value.state != VpnState.CONNECTED) continue

                val proc = aetherProcess
                debugLog(this@AetherVpnService, "watchdog #$checks: procNull=${proc == null} isAlive=${proc?.isAlive}")
                if (proc == null || !proc.isAlive) {
                    debugLog(this@AetherVpnService, "watchdog: daemon dead -> tunnel failure")
                    handleTunnelFailure("Aether core process died")
                    break
                }

                val probeOk = performSocks5LivenessProbe("127.0.0.1", 1819, 6_000)
                debugLog(this@AetherVpnService, "watchdog #$checks: socksProbe=$probeOk")
                if (!probeOk) {
                    // Confirm with a second probe after a short grace period:
                    // a single miss can be transient congestion on a
                    // high-latency GFW-throttled edge; two consecutive
                    // timeouts mean the data path is really dead.
                    delay(2_000)
                    val confirmOk = performSocks5LivenessProbe("127.0.0.1", 1819, 6_000)
                    debugLog(this@AetherVpnService, "watchdog #$checks: confirmProbe=$confirmOk")
                    if (!confirmOk) {
                        debugLog(this@AetherVpnService, "watchdog: probe failed twice -> tunnel failure")
                        handleTunnelFailure("Tunnel stopped passing traffic")
                        break
                    }
                }
                checks++
            }
        }
    }

    /**
     * Mid-session tunnel failure handler. Under GFW conditions a WireGuard
     * endpoint that validated at connect time can be blocked minutes later;
     * the daemon process survives while all traffic blackholes (the classic
     * "shows CONNECTED but nothing loads" failure). Instead of dead-ending:
     * re-hunt a fresh endpoint up to MAX_AUTO_RECONNECTS times, then surface
     * a terminal ERROR with an actionable message.
     */
    private suspend fun handleTunnelFailure(reason: String) {
        if (userInitiatedStop) {
            debugLog(this, "tunnel failure after user stop ($reason) — ignoring")
            return
        }
        if (autoReconnectAttempts < MAX_AUTO_RECONNECTS) {
            autoReconnectAttempts++
            debugLog(
                this,
                "tunnel failure: $reason -> auto-reconnect attempt $autoReconnectAttempts/$MAX_AUTO_RECONNECTS"
            )
            _vpnStatus.value = VpnStatus(
                state = VpnState.RECONNECTING,
                connectedProfile = null,
                huntingStatus = "$reason — re-hunting a fresh endpoint (attempt $autoReconnectAttempts/$MAX_AUTO_RECONNECTS)..."
            )
            updateNotification("Reconnecting (${autoReconnectAttempts}/${MAX_AUTO_RECONNECTS})…", lastProfileName)

            // Brief backoff BEFORE teardownTransport: it cancels the calling
            // watchdog job, and a cancelled coroutine must not suspend again.
            delay(2_000)

            // Tear down transport only (keep the foreground service alive);
            // startVpn performs its own teardown/port-release anyway.
            teardownTransport()

            debugLog(this, "auto-reconnect: starting fresh scan for profile=$lastProfileId")
            startVpn(lastProfileId, forceFreshScan = true)
        } else {
            debugLog(this, "tunnel failure: $reason -> ERROR after $autoReconnectAttempts reconnect attempts")
            stopVpn()
            _vpnStatus.value = VpnStatus(
                state = VpnState.ERROR,
                errorMessage = "$reason after $autoReconnectAttempts reconnect attempts. " +
                    "The network may be heavily filtered right now — try again in a moment."
            )
        }
    }

    /**
     * Real end-to-end probe through the local SOCKS5 proxy: performs the
     * SOCKS5 greeting, then a CONNECT request for a remote DOMAIN (port 80).
     * Success requires the full chain to work: local proxy -> WARP tunnel ->
     * remote resolution + egress. A dead tunnel fails the CONNECT reply or
     * times out, even though the TCP listener may still accept sockets.
     */
    private fun performSocks5LivenessProbe(host: String, port: Int, timeoutMs: Int): Boolean {
        var socket: Socket? = null
        return try {
            socket = Socket()
            socket.tcpNoDelay = true
            socket.connect(InetSocketAddress(host, port), timeoutMs)
            socket.soTimeout = timeoutMs
            val out = socket.getOutputStream()
            val input = socket.getInputStream()

            // SOCKS5 greeting: VER=5, 1 method, NO AUTHENTICATION
            out.write(byteArrayOf(0x05, 0x01, 0x00))
            out.flush()
            val greeting = ByteArray(2)
            if (readFully(input, greeting) < 2) return false
            if (greeting[0] != 0x05.toByte() || greeting[1] != 0x00.toByte()) return false

            // SOCKS5 CONNECT request with ATYP=DOMAINNAME
            val domain = "cp.cloudflare.com".toByteArray(Charsets.US_ASCII)
            val request = ByteArray(7 + domain.size)
            request[0] = 0x05
            request[1] = 0x01
            request[2] = 0x00
            request[3] = 0x03
            request[4] = domain.size.toByte()
            System.arraycopy(domain, 0, request, 5, domain.size)
            request[5 + domain.size] = ((80 ushr 8) and 0xFF).toByte()
            request[6 + domain.size] = (80 and 0xFF).toByte()
            out.write(request)
            out.flush()

            // SOCKS5 reply: VER REP RSV ATYP ...; REP == 0x00 means the remote
            // connection was established through the tunnel.
            val response = ByteArray(4)
            if (readFully(input, response) < 4) return false
            response[1] == 0x00.toByte()
        } catch (t: Throwable) {
            false
        } finally {
            try { socket?.close() } catch (ignored: Throwable) {}
        }
    }

    private fun readFully(input: InputStream, buffer: ByteArray): Int {
        var offset = 0
        while (offset < buffer.size) {
            val read = input.read(buffer, offset, buffer.size - offset)
            if (read < 0) return offset
            offset += read
        }
        return offset
    }

    private fun establishVpnInterface(settings: AppSettings): ParcelFileDescriptor? {
        // Attempt 1: Dual stack (IPv4 + IPv6) with mapped anti-censorship DNS
        try {
            val b = Builder()
                .setSession("Aether")
                .setMtu(settings.mtu)
                .addAddress("198.18.0.1", 30)
                .addRoute("0.0.0.0", 0)
                .addDnsServer("198.18.0.2")
                .addDnsServer(settings.primaryDns)

            try {
                b.addAddress("fc00::1", 120)
                b.addRoute("::", 0)
            } catch (ignored: Throwable) {}

            try {
                b.addDisallowedApplication(packageName)
            } catch (ignored: Throwable) {}

            applySplitTunneling(b, settings)
            val pfd = b.establish()
            if (pfd != null) return pfd
        } catch (e: Throwable) {
            Log.w(TAG, "Dual-stack TUN setup failed, falling back to IPv4: ${e.message}")
        }

        // Attempt 2: IPv4 Only fallback
        val b4 = Builder()
            .setSession("Aether")
            .setMtu(settings.mtu)
            .addAddress("198.18.0.1", 30)
            .addRoute("0.0.0.0", 0)
            .addDnsServer("198.18.0.2")
            .addDnsServer(settings.primaryDns)

        try {
            b4.addDisallowedApplication(packageName)
        } catch (ignored: Throwable) {}

        applySplitTunneling(b4, settings)
        return b4.establish()
    }

    private fun applySplitTunneling(builder: Builder, settings: AppSettings) {
        if (settings.selectedPackages.isEmpty()) return
        when (settings.splitTunnelMode) {
            SplitTunnelMode.BYPASS_SELECTED -> {
                for (pkg in settings.selectedPackages) {
                    try { builder.addDisallowedApplication(pkg) } catch (ignored: Throwable) {}
                }
            }
            SplitTunnelMode.ONLY_SELECTED -> {
                for (pkg in settings.selectedPackages) {
                    try { builder.addAllowedApplication(pkg) } catch (ignored: Throwable) {}
                }
            }
            SplitTunnelMode.ALL_APPS -> {}
        }
    }

    private fun isSocksPortReady(host: String, port: Int): Boolean {
        return try {
            val s = Socket()
            s.connect(InetSocketAddress(host, port), 250)
            s.close()
            true
        } catch (e: Throwable) {
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
                    val stats = try {
                        TProxyService.TProxyGetStats()
                    } catch (t: Throwable) {
                        null
                    }
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
        val notificationManager = getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        notificationManager.notify(NOTIFICATION_ID, notification)
    }

    override fun onDestroy() {
        Log.i(TAG, "onDestroy state=${_vpnStatus.value.state}")
        debugLog(this, "onDestroy state=${_vpnStatus.value.state}")
        // Preserve an ERROR status set by the connect flow or watchdog: the
        // message must survive service teardown so the user can see why the
        // connection failed. A CONNECTED/CONNECTING status, however, can no
        // longer be true once the service process is gone.
        val wasError = _vpnStatus.value.state == VpnState.ERROR
        teardownTransport()
        if (!wasError && _vpnStatus.value.state != VpnState.DISCONNECTED) {
            _vpnStatus.value = VpnStatus(state = VpnState.DISCONNECTED)
        }
        serviceScope.cancel()
        super.onDestroy()
    }
}
