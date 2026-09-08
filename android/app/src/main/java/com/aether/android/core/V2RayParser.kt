package com.aether.android.core

import android.net.Uri
import android.util.Base64
import com.aether.android.model.Profile
import com.aether.android.model.ProfileType
import com.google.gson.Gson
import com.google.gson.JsonObject
import java.net.URLDecoder
import java.nio.charset.StandardCharsets

object V2RayParser {

    private val gson = Gson()

    fun parseUri(raw: String): Profile? {
        val trimmed = raw.trim()
        return try {
            when {
                trimmed.startsWith("vless://", ignoreCase = true) -> parseVless(trimmed)
                trimmed.startsWith("vmess://", ignoreCase = true) -> parseVmess(trimmed)
                trimmed.startsWith("trojan://", ignoreCase = true) -> parseTrojan(trimmed)
                trimmed.startsWith("ss://", ignoreCase = true) -> parseShadowsocks(trimmed)
                else -> null
            }
        } catch (e: Exception) {
            null
        }
    }

    private fun parseVless(uriStr: String): Profile {
        val uri = Uri.parse(uriStr)
        val uuid = uri.userInfo ?: ""
        val host = uri.host ?: ""
        val port = uri.port.takeIf { it != -1 } ?: 443
        val name = uri.fragment?.let { URLDecoder.decode(it, "UTF-8") } ?: "VLESS Node"

        val security = uri.getQueryParameter("security") ?: "none"
        val flow = uri.getQueryParameter("flow") ?: ""
        val network = uri.getQueryParameter("type") ?: "tcp"
        val path = uri.getQueryParameter("path")?.let { URLDecoder.decode(it, "UTF-8") } ?: ""
        val sni = uri.getQueryParameter("sni") ?: ""
        val pbk = uri.getQueryParameter("pbk") ?: ""
        val sid = uri.getQueryParameter("sid") ?: ""
        val fp = uri.getQueryParameter("fp") ?: "chrome"
        val hostHeader = uri.getQueryParameter("host") ?: ""

        return Profile(
            name = name,
            type = ProfileType.VLESS,
            server = host,
            port = port,
            uuid = uuid,
            security = security,
            flow = flow,
            network = network,
            path = path,
            host = hostHeader,
            sni = sni,
            fingerprint = fp,
            publicKey = pbk,
            shortId = sid,
            rawUri = uriStr
        )
    }

    private fun parseVmess(uriStr: String): Profile {
        val b64Content = uriStr.substringAfter("vmess://")
        val decodedJson = String(Base64.decode(b64Content, Base64.DEFAULT), StandardCharsets.UTF_8)
        val json = gson.fromJson(decodedJson, JsonObject::class.java)

        val name = json.get("ps")?.asString ?: "VMess Node"
        val host = json.get("add")?.asString ?: ""
        val port = json.get("port")?.asInt ?: 443
        val id = json.get("id")?.asString ?: ""
        val net = json.get("net")?.asString ?: "tcp"
        val type = json.get("type")?.asString ?: "none"
        val hostHeader = json.get("host")?.asString ?: ""
        val path = json.get("path")?.asString ?: ""
        val tls = json.get("tls")?.asString ?: "none"
        val sni = json.get("sni")?.asString ?: hostHeader

        return Profile(
            name = name,
            type = ProfileType.VMESS,
            server = host,
            port = port,
            uuid = id,
            security = if (tls.equals("tls", ignoreCase = true)) "tls" else "none",
            network = net,
            path = path,
            host = hostHeader,
            sni = sni,
            rawUri = uriStr
        )
    }

    private fun parseTrojan(uriStr: String): Profile {
        val uri = Uri.parse(uriStr)
        val password = uri.userInfo ?: ""
        val host = uri.host ?: ""
        val port = uri.port.takeIf { it != -1 } ?: 443
        val name = uri.fragment?.let { URLDecoder.decode(it, "UTF-8") } ?: "Trojan Node"
        val sni = uri.getQueryParameter("sni") ?: host
        val network = uri.getQueryParameter("type") ?: "tcp"

        return Profile(
            name = name,
            type = ProfileType.TROJAN,
            server = host,
            port = port,
            password = password,
            security = "tls",
            network = network,
            sni = sni,
            rawUri = uriStr
        )
    }

    private fun parseShadowsocks(uriStr: String): Profile {
        val uri = Uri.parse(uriStr)
        val name = uri.fragment?.let { URLDecoder.decode(it, "UTF-8") } ?: "Shadowsocks Node"
        val host = uri.host ?: ""
        val port = uri.port.takeIf { it != -1 } ?: 8388
        val userInfo = uri.userInfo ?: ""
        val decodedUserInfo = try {
            String(Base64.decode(userInfo, Base64.DEFAULT), StandardCharsets.UTF_8)
        } catch (e: Exception) {
            userInfo
        }

        return Profile(
            name = name,
            type = ProfileType.SHADOWSOCKS,
            server = host,
            port = port,
            password = decodedUserInfo,
            rawUri = uriStr
        )
    }

    fun parseSubscription(content: String): List<Profile> {
        val decoded = try {
            val clean = content.trim().replace("\r", "").replace("\n", "")
            String(Base64.decode(clean, Base64.DEFAULT), StandardCharsets.UTF_8)
        } catch (e: Exception) {
            content
        }

        val lines = decoded.lines()
        val profiles = mutableListOf<Profile>()
        for (line in lines) {
            val p = parseUri(line)
            if (p != null) {
                profiles.add(p)
            }
        }
        return profiles
    }
}
