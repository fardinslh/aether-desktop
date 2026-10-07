package com.aether.android.core

import java.io.InputStream
import java.net.InetSocketAddress
import java.net.Socket

/** Verifies traffic through WARP, not just the local SOCKS listener. */
object Socks5TraceProbe {
    fun verify(host: String, port: Int, timeoutMs: Int, preventIranExit: Boolean): Boolean {
        return try {
            Socket().use { socket ->
                socket.connect(InetSocketAddress(host, port), timeoutMs)
                socket.soTimeout = timeoutMs
                val output = socket.getOutputStream()
                val input = socket.getInputStream()
                output.write(byteArrayOf(5, 1, 0))
                output.flush()
                if (!readExact(input, 2).contentEquals(byteArrayOf(5, 0))) return false

                val domain = "www.cloudflare.com".toByteArray(Charsets.US_ASCII)
                output.write(byteArrayOf(5, 1, 0, 3, domain.size.toByte()) + domain + byteArrayOf(0, 80))
                output.flush()
                val reply = readExact(input, 4)
                if (reply[0] != 5.toByte() || reply[1] != 0.toByte() || reply[2] != 0.toByte()) return false
                val addressSize = when (reply[3].toInt()) {
                    1 -> 4
                    4 -> 16
                    3 -> input.read().also { if (it < 0) return false }
                    else -> return false
                }
                readExact(input, addressSize + 2)

                output.write(("GET /cdn-cgi/trace HTTP/1.0\r\n" +
                    "Host: www.cloudflare.com\r\nConnection: close\r\n\r\n").toByteArray(Charsets.US_ASCII))
                output.flush()
                val bytes = java.io.ByteArrayOutputStream()
                val buffer = ByteArray(1024)
                while (true) {
                    val count = input.read(buffer)
                    if (count < 0) break
                    if (bytes.size() + count > 16_384) return false
                    bytes.write(buffer, 0, count)
                }
                val response = bytes.toString("US-ASCII")
                val bodyStart = response.indexOf("\r\n\r\n")
                if (bodyStart < 0 || !Regex("HTTP/1\\.[01] 200(?: .*?)?\\r\\n").containsMatchIn(response)) return false
                val fields = response.substring(bodyStart + 4).lineSequence()
                    .mapNotNull { line -> line.split('=', limit = 2).takeIf { it.size == 2 } }
                    .associate { it[0].trim() to it[1].trim() }
                val loc = fields["loc"].orEmpty()
                !fields["ip"].isNullOrBlank() && fields["warp"] in setOf("on", "plus") &&
                    (!preventIranExit || (loc.matches(Regex("[A-Za-z]{2}")) && !loc.equals("IR", ignoreCase = true)))
            }
        } catch (_: Exception) {
            false
        }
    }

    private fun readExact(input: InputStream, count: Int): ByteArray {
        val result = ByteArray(count)
        var offset = 0
        while (offset < count) {
            val read = input.read(result, offset, count - offset)
            if (read < 0) throw java.io.EOFException("Truncated SOCKS5 reply")
            offset += read
        }
        return result
    }
}
