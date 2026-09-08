package com.aether.android.model

import java.util.UUID

enum class ProfileType {
    WARP,
    VLESS,
    VMESS,
    TROJAN,
    SHADOWSOCKS
}

data class Profile(
    val id: String = UUID.randomUUID().toString(),
    val name: String,
    val type: ProfileType,
    val server: String,
    val port: Int,
    val uuid: String = "",
    val password: String = "",
    val security: String = "none", // none, tls, reality
    val flow: String = "",
    val network: String = "tcp", // tcp, ws, grpc
    val path: String = "",
    val host: String = "",
    val sni: String = "",
    val fingerprint: String = "chrome",
    val publicKey: String = "", // for Reality
    val shortId: String = "",   // for Reality
    val rawUri: String = "",
    var latencyMs: Long? = null,
    val createdAt: Long = System.currentTimeMillis()
)
