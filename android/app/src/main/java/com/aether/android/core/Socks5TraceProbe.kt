package com.aether.android.core
import java.io.InputStream
import java.net.InetSocketAddress
import java.net.Socket
import javax.net.ssl.SSLSocket
import javax.net.ssl.SSLSocketFactory

/** Two independent authenticated HTTPS responses through SOCKS; no direct DNS lookup. */
object Socks5TraceProbe {
    private val timer=java.util.concurrent.Executors.newSingleThreadScheduledExecutor { r -> Thread(r,"HTTPS-probe-deadline").apply { isDaemon=true } }
    fun verify(host:String,port:Int,timeoutMs:Int,preventIranExit:Boolean):Boolean = try {
        val deadline=System.nanoTime()+timeoutMs*1_000_000L
        val trace=request(host,port,"www.cloudflare.com","/cdn-cgi/trace",timeoutMs)
        val remaining=((deadline-System.nanoTime())/1_000_000).toInt()
        check(remaining>0)
        val independent=request(host,port,"www.gstatic.com","/generate_204",remaining)
        validTrace(trace,preventIranExit) && Regex("HTTP/1\\.[01] 204(?: .*?)?\\r\\n").containsMatchIn(independent)
    } catch(_:Exception) {false}
    internal fun validTrace(response:String,preventIranExit:Boolean):Boolean {
        val split=response.indexOf("\r\n\r\n")
        if(split<0 || !Regex("HTTP/1\\.[01] 200(?: .*?)?\\r\\n").containsMatchIn(response))return false
        val fields=response.substring(split+4).lineSequence().mapNotNull { it.split('=',limit=2).takeIf { it.size==2 } }.associate { it[0].trim() to it[1].trim() }
        val loc=fields["loc"].orEmpty()
        return !fields["ip"].isNullOrBlank() && (!preventIranExit || loc.matches(Regex("[A-Za-z]{2}")) && !loc.equals("IR",true))
    }
    private fun request(host:String,port:Int,domain:String,path:String,timeout:Int):String {
        Socket().use { socket ->
            val alarm=timer.schedule({ try { socket.close() } catch (_:Exception) {} },timeout.toLong(),java.util.concurrent.TimeUnit.MILLISECONDS)
            try {
            socket.connect(InetSocketAddress(host,port),timeout);socket.soTimeout=timeout
            val out=socket.getOutputStream();val input=socket.getInputStream()
            out.write(byteArrayOf(5,1,0));out.flush();check(readExact(input,2).contentEquals(byteArrayOf(5,0)))
            val name=domain.toByteArray(Charsets.US_ASCII)
            out.write(byteArrayOf(5,1,0,3,name.size.toByte())+name+byteArrayOf(1,(-69).toByte()));out.flush()
            val reply=readExact(input,4);check(reply[0]==5.toByte() && reply[1]==0.toByte() && reply[2]==0.toByte())
            val length=when(reply[3].toInt()){1->4;4->16;3->input.read().also{check(it>=0)};else->error("Invalid SOCKS address")};readExact(input,length+2)
            ((SSLSocketFactory.getDefault() as SSLSocketFactory).createSocket(socket,domain,443,true) as SSLSocket).use { tls ->
                tls.soTimeout=timeout
                val parameters=tls.sslParameters;parameters.endpointIdentificationAlgorithm="HTTPS";tls.sslParameters=parameters;tls.startHandshake()
                tls.outputStream.write("GET $path HTTP/1.0\r\nHost: $domain\r\nConnection: close\r\n\r\n".toByteArray(Charsets.US_ASCII));tls.outputStream.flush()
                val bytes=java.io.ByteArrayOutputStream();val buffer=ByteArray(1024)
                while(true){val count=tls.inputStream.read(buffer);if(count<0)break;check(bytes.size()+count<=16384);bytes.write(buffer,0,count)}
                return bytes.toString("US-ASCII")
            }
            } finally { alarm.cancel(false) }
        }
    }
    private fun readExact(input:InputStream,count:Int):ByteArray {val bytes=ByteArray(count);var offset=0;while(offset<count){val n=input.read(bytes,offset,count-offset);check(n>0){"Truncated SOCKS reply"};offset+=n};return bytes}
}
