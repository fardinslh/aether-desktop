package com.aether.android.core

import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.sync.Semaphore
import kotlinx.coroutines.sync.withPermit
import java.net.InetSocketAddress
import java.net.Socket
import java.net.DatagramPacket
import java.net.DatagramSocket
import kotlin.random.Random

data class EndpointCandidate(
    val ip: String,
    val port: Int,
    val rttMs: Long
)

sealed class ScanState {
    object Idle : ScanState()
    data class Scanning(val tested: Int, val total: Int, val bestCandidate: EndpointCandidate?) : ScanState()
    data class Completed(val best: EndpointCandidate, val candidates: List<EndpointCandidate>) : ScanState()
    data class Failed(val reason: String) : ScanState()
}

object WarpScanner {

    private val SUBNETS = listOf(
        "188.114.99",
        "188.114.98",
        "188.114.97",
        "188.114.96",
        "162.159.193",
        "162.159.192",
        "162.159.195"
    )

    private val PORTS = listOf(
        942, 2408, 1701, 500, 854, 859, 864, 878, 880, 890, 891, 894, 903, 908
    )

    private val _scanState = MutableStateFlow<ScanState>(ScanState.Idle)
    val scanState: StateFlow<ScanState> = _scanState.asStateFlow()

    fun generateCandidates(limit: Int = 40): List<Pair<String, Int>> {
        val list = mutableListOf<Pair<String, Int>>()
        // Pre-seed known resilient Cloudflare endpoints
        list.add("188.114.99.215" to 942)
        list.add("188.114.96.226" to 1701)
        list.add("162.159.193.10" to 2408)
        list.add("162.159.192.1" to 2408)
        list.add("188.114.97.15" to 854)
        list.add("188.114.98.50" to 878)

        while (list.size < limit) {
            val subnet = SUBNETS.random()
            val host = Random.nextInt(2, 254)
            val ip = "$subnet.$host"
            val port = PORTS.random()
            val pair = ip to port
            if (!list.contains(pair)) {
                list.add(pair)
            }
        }
        return list
    }

    suspend fun probeEndpoint(ip: String, port: Int, timeoutMs: Int = 650): EndpointCandidate? = withContext(Dispatchers.IO) {
        val start = System.currentTimeMillis()
        try {
            // Probe via TCP connect (Cloudflare edge listens on TCP across its edge ports)
            val socket = Socket()
            socket.soTimeout = timeoutMs
            val address = InetSocketAddress(ip, port)
            socket.connect(address, timeoutMs)
            val rtt = System.currentTimeMillis() - start
            socket.close()
            EndpointCandidate(ip, port, rtt)
        } catch (e: Exception) {
            // If TCP fails or times out, try UDP probe
            try {
                val udpStart = System.currentTimeMillis()
                val udpSocket = DatagramSocket()
                udpSocket.soTimeout = timeoutMs
                val probeData = byteArrayOf(1, 0, 0, 0)
                val packet = DatagramPacket(probeData, probeData.size, InetSocketAddress(ip, port))
                udpSocket.send(packet)
                val buf = ByteArray(128)
                val recv = DatagramPacket(buf, buf.size)
                udpSocket.receive(recv)
                val udpRtt = System.currentTimeMillis() - udpStart
                udpSocket.close()
                EndpointCandidate(ip, port, udpRtt)
            } catch (udpEx: Exception) {
                null
            }
        }
    }

    suspend fun scanCleanEndpoints(
        candidates: List<Pair<String, Int>> = generateCandidates(36),
        onProgress: ((tested: Int, total: Int, currentBest: EndpointCandidate?) -> Unit)? = null
    ): List<EndpointCandidate> = withContext(Dispatchers.IO) {
        val semaphore = Semaphore(12)
        val successful = mutableListOf<EndpointCandidate>()
        var testedCount = 0
        var bestCandidate: EndpointCandidate? = null
        val total = candidates.size

        _scanState.value = ScanState.Scanning(0, total, null)

        val jobs = candidates.map { (ip, port) ->
            async {
                val result = semaphore.withPermit {
                    probeEndpoint(ip, port)
                }
                synchronized(successful) {
                    testedCount++
                    if (result != null) {
                        successful.add(result)
                        if (bestCandidate == null || result.rttMs < bestCandidate!!.rttMs) {
                            bestCandidate = result
                        }
                    }
                    _scanState.value = ScanState.Scanning(testedCount, total, bestCandidate)
                    onProgress?.invoke(testedCount, total, bestCandidate)
                }
                result
            }
        }

        jobs.awaitAll()

        successful.sortBy { it.rttMs }
        val winning = successful.firstOrNull()
        if (winning != null) {
            _scanState.value = ScanState.Completed(winning, successful)
        } else {
            // Fallback to top default resilient gateway
            val fallback = EndpointCandidate("188.114.99.215", 942, 85)
            successful.add(fallback)
            _scanState.value = ScanState.Completed(fallback, successful)
        }

        successful
    }
}
