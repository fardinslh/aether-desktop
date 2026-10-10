package com.aether.android.ui.screens

import android.annotation.SuppressLint
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.os.PowerManager
import android.provider.Settings
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.BatteryChargingFull
import androidx.compose.material.icons.filled.Dns
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.Security
import androidx.compose.material.icons.filled.Shield
import androidx.compose.material.icons.filled.SystemUpdate
import androidx.compose.material.icons.filled.Tune
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.aether.android.model.AppSettings
import com.aether.android.model.NoizeProfile
import com.aether.android.model.ConnectionMode
import com.aether.android.model.ConnectionProfile
import com.aether.android.model.VpnProtocol
import com.aether.android.repository.AppUpdateInfo
import com.aether.android.ui.theme.*

@SuppressLint("BatteryLife")
@Composable
fun SettingsScreen(
    settings: AppSettings,
    onUpdateSettings: ((AppSettings) -> AppSettings) -> Unit,
    updateInfo: AppUpdateInfo? = null,
    isCheckingUpdates: Boolean = false,
    onCheckUpdates: () -> Unit = {},
    onDownloadUpdate: (String?) -> Unit = {},
    modifier: Modifier = Modifier
) {
    val context = LocalContext.current
    val scrollState = rememberScrollState()

    var isBatteryIgnored by remember {
        mutableStateOf(checkBatteryOptimization(context))
    }

    Column(
        modifier = modifier
            .fillMaxSize()
            .verticalScroll(scrollState)
            .padding(horizontal = 20.dp)
    ) {
        Text(
            text = "NETWORK & RESILIENCE",
            fontSize = 18.sp,
            fontWeight = FontWeight.Bold,
            fontFamily = FontFamily.Monospace,
            color = TextPrimary,
            modifier = Modifier.padding(top = 12.dp, bottom = 12.dp)
        )

        // Iran Anti-Censorship Section
        Text(
            text = "ANTI-CENSORSHIP (IRAN CARRIER MODE)",
            fontSize = 11.sp,
            fontFamily = FontFamily.Monospace,
            fontWeight = FontWeight.Bold,
            color = PrimaryCyan,
            modifier = Modifier.padding(bottom = 6.dp)
        )

        Card(
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(12.dp),
            colors = CardDefaults.cardColors(containerColor = SurfaceDark)
        ) {
            Column(modifier = Modifier.padding(14.dp)) {
                Text("CONNECTION MODE", color = TextSecondary)
                ConnectionMode.entries.forEach { mode ->
                    Row(Modifier.fillMaxWidth().clickable { onUpdateSettings { it.copy(connectionMode=mode) } }) {
                        RadioButton(selected=settings.connectionMode==mode,onClick={ onUpdateSettings { it.copy(connectionMode=mode) } })
                        Text(mode.name.replace('_',' '),color=TextSecondary)
                    }
                }
                if(settings.connectionMode==ConnectionMode.MANUAL) {
                    ConnectionProfile.entries.forEach { p ->
                        Row(Modifier.fillMaxWidth().clickable { onUpdateSettings { it.copy(manualProfile=p) } }) {
                            RadioButton(selected=settings.manualProfile==p,onClick={ onUpdateSettings { it.copy(manualProfile=p) } })
                            Text(p.id + if(p.udp) "" else " · TCP only",color=TextSecondary)
                        }
                    }
                }
                // Protocol Selector
                Text(
                    text = "CARRIER PROTOCOL",
                    fontSize = 10.sp,
                    fontFamily = FontFamily.Monospace,
                    fontWeight = FontWeight.Bold,
                    color = TextSecondary,
                    modifier = Modifier.padding(bottom = 6.dp)
                )

                val protocolOptions = listOf(
                    Triple(VpnProtocol.MASQUE_H2, "MASQUE over HTTP/2 (Recommended for Iran)", "Defeats UDP throttling via TCP 443 stream"),
                    Triple(VpnProtocol.MASQUE, "MASQUE HTTP/3", "Standard QUIC transport over UDP"),
                    Triple(VpnProtocol.WIREGUARD, "WireGuard", "Standard Cloudflare WARP protocol")
                )

                protocolOptions.forEach { (proto, title, desc) ->
                    val selected = settings.vpnProtocol == proto
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clickable { onUpdateSettings { it.copy(vpnProtocol = proto) } }
                            .padding(vertical = 6.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        Column(modifier = Modifier.weight(1f)) {
                            Text(title, color = if (selected) PrimaryCyan else TextPrimary, fontSize = 13.sp, fontWeight = FontWeight.SemiBold)
                            Text(desc, color = TextSecondary, fontSize = 11.sp)
                        }
                        RadioButton(
                            selected = selected,
                            onClick = { onUpdateSettings { it.copy(vpnProtocol = proto) } },
                            colors = RadioButtonDefaults.colors(selectedColor = PrimaryCyan, unselectedColor = BorderDark)
                        )
                    }
                }

                HorizontalDivider(modifier = Modifier.padding(vertical = 10.dp), color = BorderDark)

                // Obfuscation Profile
                Text(
                    text = "DPI OBFUSCATION (NOIZE)",
                    fontSize = 10.sp,
                    fontFamily = FontFamily.Monospace,
                    fontWeight = FontWeight.Bold,
                    color = TextSecondary,
                    modifier = Modifier.padding(bottom = 6.dp)
                )

                val noizeOptions = listOf(
                    NoizeProfile.FIREWALL to "Firewall (Recommended for Iranian DPI)",
                    NoizeProfile.GFW to "GFW (Strict Padding)",
                    NoizeProfile.NONE to "Disabled (Standard Packets)"
                )

                noizeOptions.forEach { (profile, label) ->
                    val selected = settings.noizeProfile == profile
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clickable { onUpdateSettings { it.copy(noizeProfile = profile) } }
                            .padding(vertical = 4.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        Text(label, color = if (selected) TextPrimary else TextSecondary, fontSize = 13.sp)
                        RadioButton(
                            selected = selected,
                            onClick = { onUpdateSettings { it.copy(noizeProfile = profile) } },
                            colors = RadioButtonDefaults.colors(selectedColor = PrimaryCyan, unselectedColor = BorderDark)
                        )
                    }
                }

                HorizontalDivider(modifier = Modifier.padding(vertical = 10.dp), color = BorderDark)

                OutlinedTextField(value=settings.ech.orEmpty(),onValueChange={ value -> onUpdateSettings { it.copy(ech=value.ifBlank { null }) } },label={ Text("ECH · empty disables · auto enables") },modifier=Modifier.fillMaxWidth())
                // Fragmentation Toggle
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 6.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.SpaceBetween
                ) {
                    Column(modifier = Modifier.weight(1f)) {
                        Text("TLS ClientHello Fragmentation", color = TextPrimary, fontSize = 13.sp, fontWeight = FontWeight.SemiBold)
                        Text("Splits SNI across TCP segments to bypass Deep Packet Inspection", color = TextSecondary, fontSize = 11.sp)
                    }
                    Switch(
                        checked = settings.enableFragmentation,
                        onCheckedChange = { chk -> onUpdateSettings { it.copy(enableFragmentation = chk) } },
                        colors = SwitchDefaults.colors(checkedThumbColor = PrimaryCyan, checkedTrackColor = SurfaceVariantDark)
                    )
                }

                // Prevent Iran Exit Toggle
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 6.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.SpaceBetween
                ) {
                    Column(modifier = Modifier.weight(1f)) {
                        Text("Prevent Domestic (.IR) Exit", color = TextPrimary, fontSize = 13.sp, fontWeight = FontWeight.SemiBold)
                        Text("Strictly prohibits exit nodes situated inside Iran (--exit-loc !IR)", color = TextSecondary, fontSize = 11.sp)
                    }
                    Switch(
                        checked = settings.preventIranExit,
                        onCheckedChange = { chk -> onUpdateSettings { it.copy(preventIranExit = chk) } },
                        colors = SwitchDefaults.colors(checkedThumbColor = PrimaryCyan, checkedTrackColor = SurfaceVariantDark)
                    )
                }

                // Bypass Iran Domestic Traffic Toggle
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 6.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.SpaceBetween
                ) {
                    Column(modifier = Modifier.weight(1f)) {
                        Text("Bypass Domestic Iranian Traffic", color = TextPrimary, fontSize = 13.sp, fontWeight = FontWeight.SemiBold)
                        Text("Directly routes national .ir domains and domestic banking", color = TextSecondary, fontSize = 11.sp)
                    }
                    Switch(
                        checked = settings.bypassIranTraffic,
                        onCheckedChange = { chk -> onUpdateSettings { it.copy(bypassIranTraffic = chk) } },
                        colors = SwitchDefaults.colors(checkedThumbColor = PrimaryCyan, checkedTrackColor = SurfaceVariantDark)
                    )
                }
            }
        }

        Spacer(modifier = Modifier.height(18.dp))

        // DNS Server Selector
        Text(
            text = "DNS RESOLVER",
            fontSize = 11.sp,
            fontFamily = FontFamily.Monospace,
            fontWeight = FontWeight.Bold,
            color = TextSecondary,
            modifier = Modifier.padding(bottom = 6.dp)
        )
        Card(
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(12.dp),
            colors = CardDefaults.cardColors(containerColor = SurfaceDark)
        ) {
            Column(modifier = Modifier.padding(14.dp)) {
                val dnsOptions = listOf(
                    "Cloudflare (1.1.1.1)" to "1.1.1.1",
                    "Google (8.8.8.8)" to "8.8.8.8",
                    "AdGuard DNS (94.140.14.14)" to "94.140.14.14",
                    "Quad9 (9.9.9.9)" to "9.9.9.9"
                )

                dnsOptions.forEach { (label, ip) ->
                    val selected = settings.primaryDns == ip
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clickable { onUpdateSettings { it.copy(primaryDns = ip) } }
                            .padding(vertical = 6.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Icon(Icons.Default.Dns, contentDescription = null, tint = PrimaryCyan, modifier = Modifier.size(18.dp))
                            Spacer(modifier = Modifier.width(10.dp))
                            Text(label, color = if (selected) TextPrimary else TextSecondary, fontSize = 14.sp)
                        }
                        RadioButton(
                            selected = selected,
                            onClick = { onUpdateSettings { it.copy(primaryDns = ip) } },
                            colors = RadioButtonDefaults.colors(selectedColor = PrimaryCyan, unselectedColor = BorderDark)
                        )
                    }
                }
            }
        }

        Spacer(modifier = Modifier.height(18.dp))

        // MTU Configuration
        Text(
            text = "TUNNEL MTU",
            fontSize = 11.sp,
            fontFamily = FontFamily.Monospace,
            fontWeight = FontWeight.Bold,
            color = TextSecondary,
            modifier = Modifier.padding(bottom = 6.dp)
        )
        Card(
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(12.dp),
            colors = CardDefaults.cardColors(containerColor = SurfaceDark)
        ) {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(16.dp),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Icon(Icons.Default.Tune, contentDescription = null, tint = PrimaryCyan, modifier = Modifier.size(20.dp))
                    Spacer(modifier = Modifier.width(10.dp))
                    Column {
                        Text("Maximum Transmission Unit", color = TextPrimary, fontSize = 14.sp, fontWeight = FontWeight.SemiBold)
                        Text("1500 for high-throughput TCP & HTTP/2", color = TextSecondary, fontSize = 12.sp)
                    }
                }
                Box(
                    modifier = Modifier
                        .clip(RoundedCornerShape(8.dp))
                        .background(SurfaceVariantDark)
                        .padding(horizontal = 10.dp, vertical = 6.dp)
                ) {
                    Text(
                        text = "${settings.mtu}",
                        color = PrimaryCyan,
                        fontSize = 13.sp,
                        fontFamily = FontFamily.Monospace,
                        fontWeight = FontWeight.Bold
                    )
                }
            }
        }

        Spacer(modifier = Modifier.height(18.dp))

        // Android System Battery Optimization
        Text(
            text = "BACKGROUND PERSISTENCE",
            fontSize = 11.sp,
            fontFamily = FontFamily.Monospace,
            fontWeight = FontWeight.Bold,
            color = TextSecondary,
            modifier = Modifier.padding(bottom = 6.dp)
        )
        Card(
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(12.dp),
            colors = CardDefaults.cardColors(containerColor = SurfaceDark)
        ) {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .clickable {
                        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M && !isBatteryIgnored) {
                            val intent = Intent(Settings.ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS).apply {
                                data = Uri.parse("package:${context.packageName}")
                            }
                            try {
                                context.startActivity(intent)
                            } catch (e: Exception) {
                                val fallback = Intent(Settings.ACTION_IGNORE_BATTERY_OPTIMIZATION_SETTINGS)
                                context.startActivity(fallback)
                            }
                        }
                    }
                    .padding(16.dp),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.weight(1f)) {
                    Icon(Icons.Default.BatteryChargingFull, contentDescription = null, tint = AccentPurple, modifier = Modifier.size(20.dp))
                    Spacer(modifier = Modifier.width(10.dp))
                    Column {
                        Text("Ignore Battery Optimization", color = TextPrimary, fontSize = 14.sp, fontWeight = FontWeight.SemiBold)
                        Text(
                            text = if (isBatteryIgnored) "Whitelisted (Won't disconnect in sleep)" else "Tap to grant background exemption",
                            color = if (isBatteryIgnored) StatusGreen else StatusAmber,
                            fontSize = 12.sp
                        )
                    }
                }
            }
        }

        Spacer(modifier = Modifier.height(18.dp))

        // Core Engine & App Updates Section
        Text(
            text = "UPDATES & RELEASES",
            fontSize = 11.sp,
            fontFamily = FontFamily.Monospace,
            fontWeight = FontWeight.Bold,
            color = TextSecondary,
            modifier = Modifier.padding(bottom = 6.dp)
        )
        Card(
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(12.dp),
            colors = CardDefaults.cardColors(containerColor = SurfaceDark)
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Column {
                        Text("Aether Android Release", color = TextPrimary, fontSize = 14.sp, fontWeight = FontWeight.SemiBold)
                        Text("Installed: v0.1.5 · Core: ${updateInfo?.aetherCoreVersion ?: "v2.3.0"}", color = TextSecondary, fontSize = 12.sp)
                    }
                    Button(
                        onClick = onCheckUpdates,
                        enabled = !isCheckingUpdates,
                        colors = ButtonDefaults.buttonColors(containerColor = SurfaceVariantDark),
                        contentPadding = PaddingValues(horizontal = 12.dp, vertical = 6.dp),
                        shape = RoundedCornerShape(8.dp)
                    ) {
                        if (isCheckingUpdates) {
                            CircularProgressIndicator(modifier = Modifier.size(14.dp), strokeWidth = 2.dp, color = PrimaryCyan)
                        } else {
                            Icon(Icons.Default.Refresh, contentDescription = null, tint = PrimaryCyan, modifier = Modifier.size(14.dp))
                            Spacer(modifier = Modifier.width(6.dp))
                            Text("Check", color = PrimaryCyan, fontSize = 12.sp)
                        }
                    }
                }

                if (updateInfo != null) {
                    Spacer(modifier = Modifier.height(12.dp))
                    if (updateInfo.hasUpdate) {
                        Box(
                            modifier = Modifier
                                .fillMaxWidth()
                                .clip(RoundedCornerShape(8.dp))
                                .background(PrimaryCyan.copy(alpha = 0.12f))
                                .padding(12.dp)
                        ) {
                            Column {
                                Row(
                                    modifier = Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                    verticalAlignment = Alignment.CenterVertically
                                ) {
                                    Text(
                                        text = "New Update Available: v${updateInfo.latestVersion}",
                                        color = PrimaryCyan,
                                        fontSize = 13.sp,
                                        fontWeight = FontWeight.Bold
                                    )
                                    Button(
                                        onClick = { onDownloadUpdate(updateInfo.downloadUrl) },
                                        colors = ButtonDefaults.buttonColors(containerColor = PrimaryCyan),
                                        contentPadding = PaddingValues(horizontal = 12.dp, vertical = 4.dp),
                                        shape = RoundedCornerShape(6.dp)
                                    ) {
                                        Text("Download APK", color = Color.Black, fontSize = 11.sp, fontWeight = FontWeight.Bold)
                                    }
                                }
                                if (!updateInfo.releaseNotes.isNullOrBlank()) {
                                    Spacer(modifier = Modifier.height(6.dp))
                                    Text(
                                        text = updateInfo.releaseNotes.take(150) + if (updateInfo.releaseNotes.length > 150) "..." else "",
                                        color = TextSecondary,
                                        fontSize = 11.sp
                                    )
                                }
                            }
                        }
                    } else {
                        Text(
                            text = "✓ You are running the latest version",
                            color = StatusGreen,
                            fontSize = 12.sp,
                            fontFamily = FontFamily.Monospace
                        )
                    }
                }
            }
        }

        Spacer(modifier = Modifier.height(24.dp))

        // About / Build Info
        Box(
            modifier = Modifier
                .fillMaxWidth()
                .clip(RoundedCornerShape(12.dp))
                .background(SurfaceDark)
                .padding(16.dp),
            contentAlignment = Alignment.Center
        ) {
            Column(horizontalAlignment = Alignment.CenterHorizontally) {
                Text(
                    text = "AETHER MOBILE",
                    fontFamily = FontFamily.Monospace,
                    fontWeight = FontWeight.Bold,
                    fontSize = 13.sp,
                    color = PrimaryCyan,
                    letterSpacing = 2.sp
                )
                Spacer(modifier = Modifier.height(4.dp))
                Text(
                    text = "Version 0.1.5 · Powered by Aether Core v2.3.0",
                    fontSize = 11.sp,
                    fontFamily = FontFamily.Monospace,
                    color = TextTertiary
                )
            }
        }

        Spacer(modifier = Modifier.height(30.dp))
    }
}

private fun checkBatteryOptimization(context: Context): Boolean {
    return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
        val pm = context.getSystemService(Context.POWER_SERVICE) as PowerManager
        pm.isIgnoringBatteryOptimizations(context.packageName)
    } else {
        true
    }
}
