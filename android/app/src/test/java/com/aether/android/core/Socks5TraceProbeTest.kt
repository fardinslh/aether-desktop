package com.aether.android.core

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import java.net.ServerSocket
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit

class Socks5TraceProbeTest {
    private fun probe(body: String?, preventIranExit: Boolean = true, status: Int = 200, atyp: Int = 1): Boolean {
        ServerSocket(0).use { server ->
            val executor = Executors.newSingleThreadExecutor()
            try {
                val peer = executor.submit {
                    server.accept().use { socket ->
                        socket.soTimeout = 2_000
                        val input = socket.getInputStream()
                        val output = socket.getOutputStream()
                        fun readBytes(count: Int) {
                            repeat(count) { check(input.read() >= 0) }
                        }
                        readBytes(3)
                        // Fragment greeting and reply to exercise readExact.
                        output.write(5); output.flush()
                        output.write(0); output.flush()
                        readBytes(4)
                        val domainSize = input.read()
                        readBytes(domainSize + 2)
                        output.write(byteArrayOf(5, 0, 0, atyp.toByte()))
                        when (atyp) {
                            1 -> output.write(ByteArray(4))
                            4 -> output.write(ByteArray(16))
                            3 -> output.write(byteArrayOf(3, 97, 98, 99))
                        }
                        output.write(byteArrayOf(0, 80)); output.flush()
                        if (body != null) {
                            val reader = input.bufferedReader()
                            check(reader.readLine() == "GET /cdn-cgi/trace HTTP/1.0")
                            while (!reader.readLine().isNullOrEmpty()) {}
                            output.write("HTTP/1.0 $status OK\r\n\r\n$body".toByteArray())
                            output.flush()
                        }
                    }
                }
                val result = Socks5TraceProbe.verify("127.0.0.1", server.localPort, 2_000, preventIranExit)
                peer.get(3, TimeUnit.SECONDS)
                return result
            } finally {
                executor.shutdownNow()
            }
        }
    }

    @Test fun acceptsWarpTraceAcrossReplyAddressTypes() {
        for (atyp in listOf(1, 3, 4)) {
            assertTrue(probe("ip=203.0.113.1\nwarp=on\nloc=DE\n", atyp = atyp))
        }
        assertTrue(probe("ip=203.0.113.1\nwarp=plus\nloc=DE\n"))
    }

    @Test fun rejectsListenerWithoutHttpData() {
        assertFalse(probe(null))
    }

    @Test fun rejectsIranianOrUnknownExitWhenPolicyEnabled() {
        assertFalse(probe("ip=203.0.113.1\nwarp=on\nloc=IR\n"))
        assertFalse(probe("ip=203.0.113.1\nwarp=on\n"))
        assertTrue(probe("ip=203.0.113.1\nwarp=on\nloc=IR\n", preventIranExit = false))
    }

    @Test fun rejectsNonWarpInvalidTraceAndHttpErrors() {
        assertFalse(probe("ip=203.0.113.1\nwarp=off\nloc=DE\n"))
        assertFalse(probe("warp=on\nloc=DE\n"))
        assertFalse(probe("ip=203.0.113.1\nwarp=on\nloc=DE\n", status = 503))
    }
}
