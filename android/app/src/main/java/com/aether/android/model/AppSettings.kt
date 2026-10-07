package com.aether.android.model

enum class SplitTunnelMode {
    ALL_APPS,           // Route everything
    BYPASS_SELECTED,    // Everything except selected apps
    ONLY_SELECTED       // Only route selected apps
}

enum class VpnProtocol {
    MASQUE_H2,          // MASQUE HTTP/2 (Optimal for Iran: routes via TCP 443, defeats UDP throttle)
    MASQUE,             // MASQUE HTTP/3 (Standard QUIC transport)
    WIREGUARD           // WireGuard (Default WARP protocol)
}

enum class NoizeProfile {
    FIREWALL,           // Optimal for Iran DPI bypass with MASQUE
    GFW,                // Aggressive WireGuard padding
    NONE                // Disabled
}

data class AppSettings(
    val splitTunnelMode: SplitTunnelMode = SplitTunnelMode.ALL_APPS,
    val selectedPackages: Set<String> = emptySet(),
    val primaryDns: String = "1.1.1.1",
    val secondaryDns: String = "1.0.0.1",
    val mtu: Int = 1500,
    val autoConnectOnBoot: Boolean = false,
    val enableKillSwitch: Boolean = false,
    val selectedProfileId: String? = null,
    val vpnProtocol: VpnProtocol = VpnProtocol.MASQUE_H2,
    val noizeProfile: NoizeProfile = NoizeProfile.FIREWALL,
    val enableFragmentation: Boolean = true,
    val fragmentSize: String = "10-30",
    val fragmentDelay: String = "10-20",
    val bypassIranTraffic: Boolean = true,
    val preventIranExit: Boolean = true
)
