package com.aether.android.core

import com.aether.android.model.AppSettings
import com.aether.android.model.Profile
import com.aether.android.model.ProfileType
import com.aether.android.model.SplitTunnelMode
import com.google.gson.GsonBuilder
import com.google.gson.JsonArray
import com.google.gson.JsonObject

object ConfigBuilder {

    private val gson = GsonBuilder().setPrettyPrinting().create()

    fun build(profile: Profile, settings: AppSettings): String {
        val root = JsonObject()

        // 1. Log options
        val log = JsonObject().apply {
            addProperty("level", "info")
            addProperty("timestamp", true)
        }
        root.add("log", log)

        // 2. DNS options
        val dns = JsonObject().apply {
            val servers = JsonArray().apply {
                add(JsonObject().apply {
                    addProperty("tag", "remote-dns")
                    addProperty("address", "https://${settings.primaryDns}/dns-query")
                    addProperty("detour", "proxy")
                })
                add(JsonObject().apply {
                    addProperty("tag", "direct-dns")
                    addProperty("address", settings.primaryDns)
                    addProperty("detour", "direct")
                })
            }
            add("servers", servers)

            val rules = JsonArray().apply {
                add(JsonObject().apply {
                    addProperty("outbound", "any")
                    addProperty("server", "direct-dns")
                })
            }
            add("rules", rules)
            addProperty("strategy", "prefer_ipv4")
        }
        root.add("dns", dns)

        // 3. Inbound: Virtual TUN
        val inbounds = JsonArray().apply {
            add(JsonObject().apply {
                addProperty("type", "tun")
                addProperty("tag", "tun-in")
                addProperty("interface_name", "tun0")
                addProperty("inet4_address", "172.19.0.1/30")
                addProperty("mtu", settings.mtu)
                addProperty("auto_route", true)
                addProperty("strict_route", true)
                addProperty("stack", "system")
                addProperty("sniff", true)
            })
        }
        root.add("inbounds", inbounds)

        // 4. Outbounds
        val outbounds = JsonArray().apply {
            // Main Proxy Outbound
            add(buildOutbound(profile))

            // Direct Outbound
            add(JsonObject().apply {
                addProperty("type", "direct")
                addProperty("tag", "direct")
            })

            // Block Outbound
            add(JsonObject().apply {
                addProperty("type", "block")
                addProperty("tag", "block")
            })

            // DNS Outbound
            add(JsonObject().apply {
                addProperty("type", "dns")
                addProperty("tag", "dns-out")
            })
        }
        root.add("outbounds", outbounds)

        // 5. Routing Rules
        val route = JsonObject().apply {
            val rules = JsonArray().apply {
                // Intercept DNS
                add(JsonObject().apply {
                    addProperty("protocol", "dns")
                    addProperty("outbound", "dns-out")
                })
                add(JsonObject().apply {
                    addProperty("port", 53)
                    addProperty("outbound", "dns-out")
                })

                // Private LAN Bypass
                add(JsonObject().apply {
                    val geoip = JsonArray().apply {
                        add("private")
                    }
                    add("geoip", geoip)
                    addProperty("outbound", "direct")
                })

                // Per-App Split Tunneling Rules
                if (settings.selectedPackages.isNotEmpty()) {
                    val pkgArray = JsonArray()
                    settings.selectedPackages.forEach { pkgArray.add(it) }

                    when (settings.splitTunnelMode) {
                        SplitTunnelMode.BYPASS_SELECTED -> {
                            add(JsonObject().apply {
                                add("package_name", pkgArray)
                                addProperty("outbound", "direct")
                            })
                        }
                        SplitTunnelMode.ONLY_SELECTED -> {
                            add(JsonObject().apply {
                                add("package_name", pkgArray)
                                addProperty("outbound", "proxy")
                            })
                            // If only selected, all other apps go direct
                            add(JsonObject().apply {
                                addProperty("outbound", "direct")
                            })
                        }
                        SplitTunnelMode.ALL_APPS -> {
                            // No app-specific overrides
                        }
                    }
                }
            }
            add("rules", rules)
            addProperty("auto_detect_interface", true)
            addProperty("final", "proxy")
        }
        root.add("route", route)

        return gson.toJson(root)
    }

    private fun buildOutbound(profile: Profile): JsonObject {
        val out = JsonObject().apply {
            addProperty("tag", "proxy")
            addProperty("server", profile.server)
            addProperty("server_port", profile.port)
        }

        when (profile.type) {
            ProfileType.VLESS -> {
                out.addProperty("type", "vless")
                out.addProperty("uuid", profile.uuid)
                if (profile.flow.isNotEmpty()) {
                    out.addProperty("flow", profile.flow)
                }
                if (profile.network != "tcp") {
                    val transport = JsonObject().apply {
                        addProperty("type", profile.network)
                        if (profile.path.isNotEmpty()) addProperty("path", profile.path)
                        if (profile.host.isNotEmpty()) {
                            val headers = JsonObject()
                            headers.addProperty("Host", profile.host)
                            add("headers", headers)
                        }
                    }
                    out.add("transport", transport)
                }
                if (profile.security == "tls" || profile.security == "reality") {
                    val tls = JsonObject().apply {
                        addProperty("enabled", true)
                        addProperty("server_name", if (profile.sni.isNotEmpty()) profile.sni else profile.server)
                        val utls = JsonObject().apply {
                            addProperty("enabled", true)
                            addProperty("fingerprint", if (profile.fingerprint.isNotEmpty()) profile.fingerprint else "chrome")
                        }
                        add("utls", utls)
                        if (profile.security == "reality") {
                            val reality = JsonObject().apply {
                                addProperty("enabled", true)
                                addProperty("public_key", profile.publicKey)
                                addProperty("short_id", profile.shortId)
                            }
                            add("reality", reality)
                        }
                    }
                    out.add("tls", tls)
                }
            }
            ProfileType.VMESS -> {
                out.addProperty("type", "vmess")
                out.addProperty("uuid", profile.uuid)
                out.addProperty("security", "auto")
                if (profile.security == "tls") {
                    val tls = JsonObject().apply {
                        addProperty("enabled", true)
                        addProperty("server_name", if (profile.sni.isNotEmpty()) profile.sni else profile.server)
                    }
                    out.add("tls", tls)
                }
            }
            ProfileType.TROJAN -> {
                out.addProperty("type", "trojan")
                out.addProperty("password", profile.password)
                val tls = JsonObject().apply {
                    addProperty("enabled", true)
                    addProperty("server_name", if (profile.sni.isNotEmpty()) profile.sni else profile.server)
                }
                out.add("tls", tls)
            }
            ProfileType.SHADOWSOCKS -> {
                out.addProperty("type", "shadowsocks")
                out.addProperty("method", "2022-blake3-aes-128-gcm")
                out.addProperty("password", profile.password)
            }
            ProfileType.WARP -> {
                out.addProperty("type", "wireguard")
                out.addProperty("system_interface", false)
                out.addProperty("interface_name", "warp0")
                out.addProperty("local_address", "172.16.0.2/32")
                val peers = JsonArray().apply {
                    add(JsonObject().apply {
                        addProperty("server", profile.server)
                        addProperty("server_port", profile.port)
                        addProperty("public_key", profile.publicKey)
                    })
                }
                out.add("peers", peers)
            }
        }

        return out
    }
}
