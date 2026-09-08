package com.aether.android.model

enum class VpnState {
    DISCONNECTED,
    CONNECTING,
    CONNECTED,
    RECONNECTING,
    ERROR
}

data class VpnStatus(
    val state: VpnState = VpnState.DISCONNECTED,
    val connectedProfile: Profile? = null,
    val uplinkSpeedBps: Long = 0,
    val downlinkSpeedBps: Long = 0,
    val totalUploadBytes: Long = 0,
    val totalDownloadBytes: Long = 0,
    val pingMs: Long? = null,
    val connectedSinceEpochMs: Long = 0,
    val errorMessage: String? = null,
    val huntingStatus: String? = null,
    val bestCandidateRtt: Long? = null
)
