package com.aether.android.model
import com.google.gson.annotations.SerializedName

enum class ConnectionMode { @SerializedName("auto") AUTO, @SerializedName("manual") MANUAL, @SerializedName("emergency_tor") EMERGENCY_TOR }
enum class ConnectionProfile(val id:String,val udp:Boolean,val timeoutSeconds:Long,val flags:List<String>) {
 @SerializedName("masque_h2") MASQUE_H2("masque_h2",true,60,listOf("--masque","--h2")),
 @SerializedName("masque_h3") MASQUE_H3("masque_h3",true,60,listOf("--masque","--h3")),
 @SerializedName("wireguard") WIREGUARD("wireguard",true,60,listOf("--wg")),
 @SerializedName("gool") GOOL("gool",true,60,listOf("--gool","--h2")),
 @SerializedName("gool_classic") GOOL_CLASSIC("gool_classic",true,60,listOf("--gool-classic")),
 @SerializedName("masque_in_masque") MASQUE_IN_MASQUE("masque_in_masque",true,60,listOf("--mim","--h2")),
 @SerializedName("psiphon_auto") PSIPHON_AUTO("psiphon_auto",false,180,listOf("--psiphon-only","--psiphon-mode","auto")),
 @SerializedName("psiphon_cdn") PSIPHON_CDN("psiphon_cdn",false,180,listOf("--psiphon-only","--psiphon-mode","cdn")),
 @SerializedName("psiphon_reverse") PSIPHON_REVERSE("psiphon_reverse",true,240,listOf("--masque","--h2","--psiphon-reverse")),
 @SerializedName("tor") TOR("tor",false,420,listOf("--tor-only","--tor-bridges"));
 fun manualAttempt(deepScan: Boolean) = Triple(this, if(deepScan) maxOf(timeoutSeconds,340L) else timeoutSeconds, !deepScan)
 companion object { fun autoAttempts(cached:ConnectionProfile?) = buildList { if(cached!=null && cached!=TOR)add(Triple(cached,25L,true)); for(p in listOf(MASQUE_H2,MASQUE_H3,PSIPHON_AUTO,PSIPHON_CDN))add(Triple(p,p.timeoutSeconds,false)) } }
}
