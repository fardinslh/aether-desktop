package com.aether.android.model

enum class SplitTunnelMode {
    ALL_APPS,           // Route everything
    BYPASS_SELECTED,    // Everything except selected apps
    ONLY_SELECTED       // Only route selected apps
}

data class AppSettings(
    val splitTunnelMode: SplitTunnelMode = SplitTunnelMode.ALL_APPS,
    val selectedPackages: Set<String> = emptySet(),
    val primaryDns: String = "1.1.1.1",
    val secondaryDns: String = "1.0.0.1",
    val mtu: Int = 1500,
    val autoConnectOnBoot: Boolean = false,
    val enableKillSwitch: Boolean = false,
    val selectedProfileId: String? = null
)
