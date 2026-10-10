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
import com.aether.android.core.Socks5TraceProbe
import com.aether.android.model.ConnectionMode
import com.aether.android.model.ConnectionProfile
import com.aether.android.model.AppSettings
import com.aether.android.model.NoizeProfile
import com.aether.android.model.Profile
import com.aether.android.model.SplitTunnelMode
import com.aether.android.model.VpnProtocol
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
                autoReconnectAttempts = 0
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
                autoReconnectAttempts = 0
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
        val previousConnectJob = connectJob
        previousConnectJob?.cancel()

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
                // Cancellation alone does not finish a blocking process/socket
                // operation. Wait for the previous attempt before replacing it.
                previousConnectJob?.join()
                ensureActive()
                // 0. Tear down any previous transport and free port 1819.
                //    Without this, a leaked daemon from an earlier attempt keeps
                //    port 1819 open, the readiness check below passes instantly
                //    against the OLD process, and the session is declared
                //    CONNECTED while routing into a dead tunnel.
                Log.i(TAG, "connect[gen]: teardown of previous transport")
                teardownTransport(keepTun = vpnInterface != null)

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

                val selectedTransport = selectTransport(settings, forceFreshScan)
                val activeCleanIp = selectedTransport.id
                val activeCleanPort = 1819
                val activeCleanRtt = _vpnStatus.value.bestCandidateRtt ?: 0L
                ensureActive()
                // 5. Establish Android VpnService TUN Interface with MapDNS (anti-censorship)
                vpnInterface = vpnInterface ?: establishVpnInterface(settings)
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
                    if (!tproxyRunning) { TProxyService.TProxyStartService(tproxyConf.absolutePath, tunFd); tproxyRunning = true }
                } catch (t: Throwable) {
                    Log.e(TAG, "TProxyStartService native failure", t)
                    debugLog(this@AetherVpnService, "connect: TProxyStartService FAILED: ${t.message}")
                    throw IllegalStateException("Failed to start in-process TProxy engine: ${t.message}")
                }
                Log.i(TAG, "connect[gen]: TProxy engine started, CONNECTED")
                debugLog(this@AetherVpnService, "connect: TProxy started, CONNECTED ip=$activeCleanIp:$activeCleanPort rtt=$activeCleanRtt")

                // 7. Transition to CONNECTED
                val activeConnectedProfile = profile.copy(
                    server = activeCleanIp,
                    port = activeCleanPort,
                    latencyMs = activeCleanRtt
                )

                _vpnStatus.value = _vpnStatus.value.copy(
                    activeProfile = selectedTransport,
                    supportsUdp = selectedTransport.udp,
                    state = VpnState.CONNECTED,
                    connectedProfile = activeConnectedProfile,
                    pingMs = activeCleanRtt,
                    huntingStatus = if (selectedTransport.udp) "Connected · ${selectedTransport.id}" else "Connected · ${selectedTransport.id} · TCP only",
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
                ensureActive()
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
                    val failedStatus = _vpnStatus.value
                    if (vpnInterface == null) stopVpn() else { stopAetherTree(); updateNotification("Connection failed · protected traffic blocked",lastProfileName) }
                    _vpnStatus.value = failedStatus.copy(
                        state = VpnState.ERROR,
                        failureReason = t.message ?: "Connection error: ${t.javaClass.simpleName}",
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
    private var tproxyRunning = false
    private suspend fun selectTransport(settings: AppSettings, forceFreshScan: Boolean): ConnectionProfile {
        val nativeDir = File(applicationInfo.nativeLibraryDir)
        val manifest = org.json.JSONObject(assets.open("runtime/${Build.SUPPORTED_ABIS.first()}.json").bufferedReader().use { it.readText() })
        for (name in manifest.keys()) {
            val f = File(nativeDir,name)
            val hash = java.security.MessageDigest.getInstance("SHA-256").digest(f.readBytes()).joinToString("") { "%02x".format(it.toInt() and 255) }
            check(hash == manifest.getString(name)) { "Runtime checksum mismatch: $name" }
        }
        val connectivity = getSystemService(Context.CONNECTIVITY_SERVICE) as android.net.ConnectivityManager
        val network = connectivity.allNetworks.firstOrNull { connectivity.getNetworkCapabilities(it)?.hasCapability(android.net.NetworkCapabilities.NET_CAPABILITY_NOT_VPN) == true }
        val key = network?.let { n ->
            val links = connectivity.getLinkProperties(n)
            val raw = "${links?.interfaceName}:${links?.routes?.filter { it.isDefaultRoute }?.map { it.gateway }}"
            java.security.MessageDigest.getInstance("SHA-256").digest(raw.toByteArray()).joinToString("") { "%02x".format(it.toInt() and 255) }
        }
        val cache = key?.let { File(filesDir,"route-$it") }
        val cached = cache?.takeIf { it.exists() }?.readText()?.let { id -> ConnectionProfile.entries.firstOrNull { it.id == id } }
        val manual = settings.manualProfile ?: when(settings.vpnProtocol) { VpnProtocol.MASQUE_H2 -> ConnectionProfile.MASQUE_H2; VpnProtocol.MASQUE -> ConnectionProfile.MASQUE_H3; VpnProtocol.WIREGUARD -> ConnectionProfile.WIREGUARD }
        val attempts = when(settings.connectionMode) {
            ConnectionMode.AUTO -> ConnectionProfile.autoAttempts(if(forceFreshScan) null else cached)
            ConnectionMode.MANUAL -> listOf(manual.manualAttempt(forceFreshScan))
            ConnectionMode.EMERGENCY_TOR -> listOf(Triple(ConnectionProfile.TOR,420L,false))
        }
        val legacy = File(filesDir,"aether.toml")
        // Only invalidate the old shared installation identity; keep individually registered keys.
        if(legacy.exists() && legacy.readText().contains("4df2f838-0671-44db-9850-fe63f2a01741")) legacy.delete()
        val failures = mutableListOf<String>()
        for ((profile,seconds,quick) in attempts) {
            currentCoroutineContext().ensureActive()
            candidateMonitoringJob?.cancel()
            stopAetherTree()
            _vpnStatus.value = _vpnStatus.value.copy(activeProfile=profile,supportsUdp=profile.udp,huntingStatus="Trying ${profile.id}",failureReason=null)
            val dir=File(filesDir,"transports/${profile.id}").apply { mkdirs() }
            val config=File(dir,"aether.toml")
            if(!config.exists() && legacy.exists() && legacy.length()>50) legacy.copyTo(config)
            val cmd=mutableListOf(File(nativeDir,"libaether.so").absolutePath,"--config",config.absolutePath,"--bind","127.0.0.1:1819","-4")
            cmd.addAll(profile.flags)
            cmd.add(if(quick) "--quick-reconnect" else "--no-quick-reconnect")
            if(!quick) cmd.add(if(settings.connectionMode == ConnectionMode.MANUAL && forceFreshScan) "--thorough" else "--turbo")
            if(settings.enableFragmentation && profile in listOf(ConnectionProfile.MASQUE_H2,ConnectionProfile.PSIPHON_REVERSE)) {
                cmd.addAll(listOf("--fragment","--fragment-size",settings.fragmentSize,"--fragment-delay",settings.fragmentDelay))
            }
            settings.ech?.takeIf { it.isNotBlank() }?.let { cmd.addAll(listOf("--ech",it)) }
            val psiphon=File(nativeDir,"libpsiphon_tunnel_core.so")
            if(psiphon.exists()) cmd.addAll(listOf("--psiphon-bin",psiphon.absolutePath))
            for ((name,bin) in listOf("lyrebird" to "liblyrebird.so","snowflake" to "libsnowflake_client.so","webtunnel" to "libwebtunnel.so")) {
                val f=File(nativeDir,bin);if(f.exists())cmd.addAll(listOf("--tor-pt","$name=${f.absolutePath}"))
            }
            try {
                val pidFile=File(dir,"owned-process.pid").apply { delete() }
                // Constant shell shim records the owned PID before exec; arguments stay positional.
                val launch=listOf("/system/bin/sh","-c","echo \$\$ > \"\$1\"; shift; exec \"\$@\"","aether-launch",pidFile.absolutePath)+cmd
                val proc=ProcessBuilder(launch).directory(dir).redirectErrorStream(true).start()
                aetherProcess=proc
                repeat(20) { if(!pidFile.exists())delay(10) }
                ownedAetherPid=pidFile.takeIf { it.exists() }?.readText()?.trim()?.toIntOrNull()
                check(ownedAetherPid != null) { "Unable to track the transport process" }
                proc.outputStream.close()
                candidateMonitoringJob=serviceScope.launch {
                    try { proc.inputStream.bufferedReader().useLines { lines -> lines.forEach { line ->
                        Log.d(TAG,line)
                        parseCandidateFromLine(line)?.let { (_,rtt) -> _vpnStatus.value=_vpnStatus.value.copy(bestCandidateRtt=rtt) }
                    } } } catch (_:Throwable) {}
                }
                val deadline=System.currentTimeMillis()+seconds*1000
                while(System.currentTimeMillis()<deadline) {
                    currentCoroutineContext().ensureActive()
                    check(proc.isAlive) { "Core process exited" }
                    if(isSocksPortReady("127.0.0.1",1819) && performSocks5LivenessProbe("127.0.0.1",1819,minOf(6_000L,deadline-System.currentTimeMillis()).coerceAtLeast(1).toInt())) { cache?.writeText(profile.id);return profile }
                    delay(500)
                }
                error("${profile.id} exceeded ${seconds}s")
            } catch(e:CancellationException) { stopAetherTree();throw e }
            catch(e:Exception) { failures.add("${profile.id}: ${e.message}");_vpnStatus.value=_vpnStatus.value.copy(failureReason=e.message);stopAetherTree() }
        }
        error(failures.joinToString("; "))
    }
    private var ownedAetherPid: Int? = null
    private fun stopAetherTree() {
        val proc=aetherProcess ?: return
        val pid=ownedAetherPid
        val children=mutableListOf<Int>()
        fun collect(parent:Int) {
            val tasks=File("/proc/$parent/task").listFiles().orEmpty()
            for(task in tasks) {
                val ids=runCatching { File(task,"children").readText().trim().split(Regex("\\s+")).mapNotNull { it.toIntOrNull() } }.getOrDefault(emptyList())
                for(id in ids) if(id !in children) {
                    children.add(id)
                    android.os.Process.sendSignal(id,19) // Freeze owned children before collecting descendants.
                    collect(id)
                }
            }
        }
        if(pid!=null && proc.isAlive) {
            android.os.Process.sendSignal(pid,19)
            collect(pid)
        }
        // Helpers can survive an unexpected core exit and be reparented. Match this
        // installation's exact native paths and UID, never a global process name.
        val nativeDir=File(applicationInfo.nativeLibraryDir)
        val helperPaths=setOf("libpsiphon_tunnel_core.so","liblyrebird.so","libsnowflake_client.so","libwebtunnel.so").map { File(nativeDir,it).absolutePath }.toSet()
        for(entry in File("/proc").listFiles().orEmpty()) {
            val helperPid=entry.name.toIntOrNull() ?: continue
            val owned=runCatching {
                android.system.Os.stat(entry.absolutePath).st_uid==android.os.Process.myUid() && File(entry,"cmdline").readText().substringBefore('\u0000') in helperPaths
            }.getOrDefault(false)
            if(owned)android.os.Process.killProcess(helperPid)
        }
        for(child in children.asReversed()) android.os.Process.killProcess(child)
        proc.destroyForcibly()
        runCatching { proc.waitFor(1500,java.util.concurrent.TimeUnit.MILLISECONDS) }
        aetherProcess=null;ownedAetherPid=null
    }
    private fun teardownTransport(keepTun: Boolean = false) {
        healthWatchdogJob?.cancel()
        healthWatchdogJob = null
        speedMonitoringJob?.cancel()
        speedMonitoringJob = null
        candidateMonitoringJob?.cancel()
        candidateMonitoringJob = null

        try {
            if (!keepTun) { TProxyService.TProxyStopService(); tproxyRunning = false }
        } catch (ignored: Throwable) {}

        stopAetherTree()

        try {
            if (!keepTun) vpnInterface?.close()
        } catch (ignored: Throwable) {}
        if (!keepTun) vpnInterface = null
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
            var checks = 0
            var failures = 0
            while (isActive) {
                delay(15_000)
                if (_vpnStatus.value.state != VpnState.CONNECTED) continue
                val ok = aetherProcess?.isAlive == true && performSocks5LivenessProbe("127.0.0.1",1819,6_000)
                failures = if (ok) 0 else failures + 1
                if (failures >= 3) { handleTunnelFailure("Three consecutive HTTPS health failures"); break }
                checks++
                // A briefly live listener must not reset the retry budget.
                if (checks >= 6) autoReconnectAttempts = 0
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
            _vpnStatus.value = _vpnStatus.value.copy(
                failureReason = reason,
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
            teardownTransport(keepTun = true)

            debugLog(this, "auto-reconnect: starting fresh scan for profile=$lastProfileId")
            startVpn(lastProfileId, forceFreshScan = false)
        } else {
            debugLog(this, "tunnel failure: $reason -> ERROR after $autoReconnectAttempts reconnect attempts")
            val exhaustedAttempts = autoReconnectAttempts
            stopAetherTree()
            updateNotification("Connection failed · protected traffic blocked",lastProfileName)
            _vpnStatus.value = _vpnStatus.value.copy(
                state = VpnState.ERROR,
                failureReason = reason,
                errorMessage = "$reason after $exhaustedAttempts reconnect attempts. " +
                    "The network may be heavily filtered right now — try again in a moment."
            )
        }
    }

    /**
     * Real end-to-end probe through the local SOCKS5 proxy: performs the
     * SOCKS5 greeting, DOMAIN CONNECT and an HTTP Cloudflare WARP trace.
     * A successful CONNECT reply alone does not prove usable traffic.
     */
    private fun performSocks5LivenessProbe(host: String, port: Int, timeoutMs: Int): Boolean {
        return Socks5TraceProbe.verify(
            host, port, timeoutMs,
            AetherApplication.instance.settingsRepository.settings.value.preventIranExit
        )
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

        applySplitTunneling(b4, settings)
        return b4.establish()
    }

    private fun applySplitTunneling(builder: Builder, settings: AppSettings) {
        if (settings.splitTunnelMode == SplitTunnelMode.ONLY_SELECTED) {
            val packages = settings.selectedPackages.filter { it != packageName }
            require(packages.isNotEmpty()) { "Select at least one app for Only Selected mode" }
            // Android forbids mixing allowed and disallowed application lists.
            // Omitting our package from the allowlist keeps the daemon outside TUN.
            packages.forEach { builder.addAllowedApplication(it) }
            return
        }
        builder.addDisallowedApplication(packageName)
        when (settings.splitTunnelMode) {
            SplitTunnelMode.BYPASS_SELECTED -> {
                for (pkg in settings.selectedPackages) {
                    try { builder.addDisallowedApplication(pkg) } catch (ignored: Throwable) {}
                }
            }
            SplitTunnelMode.ONLY_SELECTED -> error("Only Selected mode already handled")
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
