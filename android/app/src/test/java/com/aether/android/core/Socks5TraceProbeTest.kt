package com.aether.android.core
import org.junit.Assert.*
import org.junit.Test
class Socks5TraceProbeTest {
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
