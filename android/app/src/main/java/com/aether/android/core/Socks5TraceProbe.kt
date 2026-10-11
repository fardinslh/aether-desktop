package com.aether.android.core

import java.io.ByteArrayOutputStream
import java.io.InputStream
import java.net.InetSocketAddress
import java.net.Socket
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import javax.net.ssl.SSLSocket
import javax.net.ssl.SSLSocketFactory

/** Two authenticated HTTPS responses through SOCKS; DNS stays inside the proxy. */
object Socks5TraceProbe {
    const val TIMEOUT_MS = 30_000
    private const val REQUEST_TIMEOUT_MS = 15_000
    private const val MAX_RESPONSE_BYTES = 16_384
    private val timer = Executors.newSingleThreadScheduledExecutor { runnable ->
        Thread(runnable, "HTTPS-probe-deadline").apply { isDaemon = true }
    }

    fun verify(host: String, port: Int, timeoutMs: Int, preventIranExit: Boolean): Boolean =
        verifyDetailed(host, port, timeoutMs, preventIranExit).isSuccess

    fun verifyDetailed(
        host: String, port: Int, timeoutMs: Int, preventIranExit: Boolean,
        isActive: () -> Boolean = { true }
    ): Result<Unit> = runCatching {
        require(timeoutMs > 0) { "HTTPS probe deadline expired" }
        val deadline = System.nanoTime() + timeoutMs * 1_000_000L
        fun remaining(): Int = ((deadline - System.nanoTime()) / 1_000_000)
            .coerceAtMost(REQUEST_TIMEOUT_MS.toLong()).toInt().also {
                check(it > 0) { "HTTPS probe deadline expired" }
                check(isActive()) { "HTTPS probe cancelled" }
            }
        val trace = request(host, port, "www.cloudflare.com", "/cdn-cgi/trace", remaining(), isActive)
        check(validTrace(trace, preventIranExit)) { "HTTPS trace failed or exit country was rejected" }
        val independent = request(host, port, "www.gstatic.com", "/generate_204", remaining(), isActive)
        check(statusCode(independent) == 204) { "Independent HTTPS probe did not return 204" }
    }

    internal fun validTrace(response: String, preventIranExit: Boolean): Boolean {
        val split = response.indexOf("\r\n\r\n")
        if (split < 0 || statusCode(response) != 200) return false
        val fields = response.substring(split + 4).lineSequence().mapNotNull {
            it.split('=', limit = 2).takeIf { parts -> parts.size == 2 }
        }.associate { it[0].trim() to it[1].trim() }
        val loc = fields["loc"].orEmpty()
        return !fields["ip"].isNullOrBlank() &&
            (!preventIranExit || loc.matches(Regex("[A-Za-z]{2}")) && !loc.equals("IR", true))
    }

    private fun statusCode(response: String): Int? =
        Regex("^HTTP/1\\.[01] ([0-9]{3})(?: [^\\r\\n]*)?\\r\\n")
            .find(response)?.groupValues?.get(1)?.toIntOrNull()

    private fun request(host: String, port: Int, domain: String, path: String, timeout: Int, isActive: () -> Boolean): String {
        Socket().use { socket ->
            val alarm = timer.schedule({ runCatching { socket.close() } }, timeout.toLong(), TimeUnit.MILLISECONDS)
            // Coroutine cancellation does not interrupt Java socket reads.
            val cancellation = timer.scheduleAtFixedRate({
                if (!isActive()) runCatching { socket.close() }
            }, 100, 100, TimeUnit.MILLISECONDS)
            try {
                socket.connect(InetSocketAddress(host, port), timeout)
                socket.soTimeout = timeout
                val out = socket.getOutputStream()
                val input = socket.getInputStream()
                out.write(byteArrayOf(5, 1, 0))
                out.flush()
                check(readExact(input, 2).contentEquals(byteArrayOf(5, 0))) { "SOCKS greeting rejected" }
                val name = domain.toByteArray(Charsets.US_ASCII)
                out.write(byteArrayOf(5, 1, 0, 3, name.size.toByte()) + name + byteArrayOf(1, 187.toByte()))
                out.flush()
                val reply = readExact(input, 4)
                check(reply[0] == 5.toByte() && reply[1] == 0.toByte() && reply[2] == 0.toByte()) {
                    "SOCKS CONNECT rejected (reply ${reply[1].toInt() and 255})"
                }
                val length = when (reply[3].toInt()) {
                    1 -> 4
                    4 -> 16
                    3 -> input.read().also { check(it >= 0) { "Truncated SOCKS reply" } }
                    else -> error("Invalid SOCKS address")
                }
                readExact(input, length + 2)
                val factory = SSLSocketFactory.getDefault() as SSLSocketFactory
                (factory.createSocket(socket, domain, 443, true) as SSLSocket).use { tls ->
                    tls.soTimeout = timeout
                    val parameters = tls.sslParameters
                    parameters.endpointIdentificationAlgorithm = "HTTPS"
                    tls.sslParameters = parameters
                    tls.startHandshake()
                    tls.outputStream.write("GET $path HTTP/1.0\r\nHost: $domain\r\nConnection: close\r\n\r\n".toByteArray(Charsets.US_ASCII))
                    tls.outputStream.flush()
                    return readResponse(tls.inputStream)
                }
            } catch (e: Exception) {
                throw IllegalStateException("$domain: ${e.message ?: e.javaClass.simpleName}", e)
            } finally {
                alarm.cancel(false)
                cancellation.cancel(false)
            }
        }
    }

    /** Finish at HTTP message boundaries; a live TLS connection need not close. */
    internal fun readResponse(input: InputStream): String {
        var wireBytes = 0
        fun line(): String {
            val bytes = ByteArrayOutputStream()
            while (true) {
                val b = input.read()
                check(b >= 0) { "Truncated HTTP headers" }
                check(++wireBytes <= MAX_RESPONSE_BYTES) { "HTTPS response too large" }
                bytes.write(b)
                if (b == 10) {
                    val text = bytes.toString("US-ASCII")
                    check(text.endsWith("\r\n")) { "Invalid HTTP line ending" }
                    return text.dropLast(2)
                }
            }
        }
        val header = StringBuilder(line()).append("\r\n")
        val fields = mutableMapOf<String, String>()
        while (true) {
            val value = line()
            header.append(value).append("\r\n")
            if (value.isEmpty()) break
            val parts = value.split(':', limit = 2)
            check(parts.size == 2) { "Invalid HTTP header" }
            val key = parts[0].trim().lowercase()
            check(key !in fields || key !in setOf("content-length", "transfer-encoding")) { "Duplicate HTTP framing header" }
            fields[key] = parts[1].trim()
        }
        val code = statusCode(header.toString()) ?: error("Invalid HTTP status")
        // Error responses cannot verify the tunnel, and 204 has no message body.
        if (code != 200) return header.toString()
        val body = ByteArrayOutputStream()
        fun append(count: Int) {
            check(count >= 0 && count <= MAX_RESPONSE_BYTES - wireBytes) { "HTTPS response too large" }
            body.write(readExact(input, count))
            wireBytes += count
        }
        when {
            fields.containsKey("transfer-encoding") -> {
                check(fields["transfer-encoding"].equals("chunked", true)) { "Unsupported HTTP transfer encoding" }
                while (true) {
                    val size = line().substringBefore(';').trim().toIntOrNull(16)
                        ?: error("Invalid HTTP chunk size")
                    if (size == 0) {
                        while (line().isNotEmpty()) { /* bounded trailers */ }
                        break
                    }
                    append(size)
                    check(line().isEmpty()) { "Invalid HTTP chunk ending" }
                }
            }
            fields.containsKey("content-length") -> append(
                fields.getValue("content-length").toIntOrNull() ?: error("Invalid HTTP content length")
            )
            else -> {
                while (true) {
                    val b = input.read()
                    if (b < 0) break
                    check(++wireBytes <= MAX_RESPONSE_BYTES) { "HTTPS response too large" }
                    body.write(b)
                }
            }
        }
        return header.toString() + body.toString("US-ASCII")
    }

    private fun readExact(input: InputStream, count: Int): ByteArray {
        val bytes = ByteArray(count)
        var offset = 0
        while (offset < count) {
            val n = input.read(bytes, offset, count - offset)
            check(n > 0) { "Truncated response" }
            offset += n
        }
        return bytes
    }
}
