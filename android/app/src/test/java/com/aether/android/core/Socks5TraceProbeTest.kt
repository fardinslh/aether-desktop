package com.aether.android.core
import org.junit.Assert.*
import org.junit.Test
class Socks5TraceProbeTest {
 private fun openConnection(response:String)=object:java.io.ByteArrayInputStream(response.toByteArray(Charsets.US_ASCII)) {
  override fun read():Int { check(available()>0) { "Probe waited for connection close after a complete response" };return super.read() }
  override fun read(bytes:ByteArray,offset:Int,length:Int):Int { check(available()>0) { "Probe waited for connection close after a complete response" };return super.read(bytes,offset,length) }
 }
 @Test fun fixedLengthTraceCompletesWithoutWaitingForTlsClose(){
  val body="ip=203.0.113.1\nloc=DE\n"
  val response="HTTP/1.1 200 OK\r\nContent-Length: ${body.length}\r\n\r\n$body"
  assertTrue(Socks5TraceProbe.validTrace(Socks5TraceProbe.readResponse(openConnection(response)),true))
 }
 @Test fun independent204CompletesAtHeadersWithoutWaitingForTlsClose(){
  val response="HTTP/1.1 204 No Content\r\nConnection: keep-alive\r\n\r\n"
  assertEquals(response,Socks5TraceProbe.readResponse(openConnection(response)))
 }
 @Test fun chunkedTraceIsDecodedBeforeExitCountryVerification(){
  val body="ip=203.0.113.1\nloc=DE\n"
  val response="HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n${body.length.toString(16)}\r\n$body\r\n0\r\n\r\n"
  assertTrue(Socks5TraceProbe.validTrace(Socks5TraceProbe.readResponse(openConnection(response)),true))
 }
 @Test fun truncatedAndOversizedResponsesCannotPass(){
  for(response in listOf("HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\nip=1\n", "HTTP/1.1 200 OK\r\nContent-Length: 999999\r\n\r\n", "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n-1\r\n")) {
   assertTrue(runCatching { Socks5TraceProbe.readResponse(response.byteInputStream()) }.isFailure)
  }
 }
 @Test fun httpStatusInBodyCannotOverrideFailureStatus(){
  assertFalse(Socks5TraceProbe.validTrace("HTTP/1.0 503 Unavailable\r\n\r\nHTTP/1.0 200 OK\r\nip=203.0.113.1\nloc=DE\n",true))
 }
 @Test fun cancelledAttemptClosesBlockedSocksReadBeforeItsDeadline(){
  java.net.ServerSocket(0).use { server ->
   val accepted=java.util.concurrent.CountDownLatch(1)
   val active=java.util.concurrent.atomic.AtomicBoolean(true)
   val finished=java.util.concurrent.CountDownLatch(1)
   val result=java.util.concurrent.atomic.AtomicBoolean(true)
   val peer=Thread { server.accept().use { socket ->
    socket.soTimeout=3000
    val input=socket.getInputStream()
    repeat(3) { input.read() }
    accepted.countDown()
    while(input.read()!=-1) { /* hold the greeting reply until cancellation */ }
   } }
   val probe=Thread {
    result.set(Socks5TraceProbe.verifyDetailed("127.0.0.1",server.localPort,10_000,true) { active.get() }.isSuccess)
    finished.countDown()
   }
   peer.start();probe.start()
   assertTrue(accepted.await(2,java.util.concurrent.TimeUnit.SECONDS))
   active.set(false)
   assertTrue(finished.await(2,java.util.concurrent.TimeUnit.SECONDS))
   assertFalse(result.get())
   peer.join(3000);probe.join(3000)
  }
 }
 private fun trace(body:String,status:Int=200)="HTTP/1.0 $status OK\r\n\r\n$body"
 @Test fun allowsPublicTcpTransportWithoutWarpMarker(){assertTrue(Socks5TraceProbe.validTrace(trace("ip=203.0.113.1\nwarp=off\nloc=DE\n"),true))}
 @Test fun enforcesExitPolicy(){assertFalse(Socks5TraceProbe.validTrace(trace("ip=203.0.113.1\nloc=IR\n"),true));assertFalse(Socks5TraceProbe.validTrace(trace("ip=203.0.113.1\n"),true));assertTrue(Socks5TraceProbe.validTrace(trace("ip=203.0.113.1\nloc=IR\n"),false))}
 @Test fun rejectsCaptiveAndErrorResponses(){assertFalse(Socks5TraceProbe.validTrace(trace("<html>Portal</html>"),false));assertFalse(Socks5TraceProbe.validTrace(trace("ip=203.0.113.1\nloc=DE\n",503),true))}
 @Test fun rejectsCleartextAfterSuccessfulSocksConnect(){
  java.net.ServerSocket(0).use { server ->
   val thread=Thread {
    server.accept().use { socket ->
     socket.soTimeout=1000
     val input=socket.getInputStream();val out=socket.getOutputStream()
     repeat(3){input.read()};out.write(byteArrayOf(5,0));out.flush()
     repeat(4){input.read()};val n=input.read();repeat(n){input.read()}
     check(input.read()==1 && input.read()==187) // 443
     out.write(byteArrayOf(5,0,0,1,0,0,0,0,0,0));out.flush()
     out.write("HTTP/1.0 200 OK\r\n\r\nip=203.0.113.1\nloc=DE\n".toByteArray());out.flush()
    }
   }
   thread.start();assertFalse(Socks5TraceProbe.verify("127.0.0.1",server.localPort,1000,true));thread.join(2000)
  }
 }
}
