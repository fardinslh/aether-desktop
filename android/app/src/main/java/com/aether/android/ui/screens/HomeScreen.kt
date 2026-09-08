package com.aether.android.ui.screens

import androidx.compose.animation.core.*
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowDownward
import androidx.compose.material.icons.filled.ArrowUpward
import androidx.compose.material.icons.filled.PowerSettingsNew
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.Speed
import androidx.compose.material.icons.filled.TravelExplore
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.scale
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.aether.android.model.Profile
import com.aether.android.model.VpnState
import com.aether.android.model.VpnStatus
import com.aether.android.ui.theme.*

@Composable
fun HomeScreen(
    status: VpnStatus,
    activeProfile: Profile?,
    onToggleConnect: () -> Unit,
    onNavigateToServers: () -> Unit,
    onRescanGateway: () -> Unit = {},
    modifier: Modifier = Modifier
) {
    val isConnected = status.state == VpnState.CONNECTED
    val isConnecting = status.state == VpnState.CONNECTING || status.state == VpnState.RECONNECTING

    // Pulsing animation for the connect ring
    val infiniteTransition = rememberInfiniteTransition(label = "pulse")
    val pulseScale by infiniteTransition.animateFloat(
        initialValue = 1f,
        targetValue = if (isConnected || isConnecting) 1.08f else 1f,
        animationSpec = infiniteRepeatable(
            animation = tween(1200, easing = FastOutSlowInEasing),
            repeatMode = RepeatMode.Reverse
        ),
        label = "pulse_scale"
    )

    Column(
        modifier = modifier
            .fillMaxSize()
            .padding(horizontal = 24.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.SpaceBetween
    ) {
        Spacer(modifier = Modifier.height(16.dp))

        // Active Node Card
        Card(
            modifier = Modifier
                .fillMaxWidth()
                .clickable { onNavigateToServers() },
            shape = RoundedCornerShape(16.dp),
            colors = CardDefaults.cardColors(containerColor = SurfaceDark)
        ) {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(16.dp),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Column {
                    Text(
                        text = if (isConnected) "ACTIVE CLEAN GATEWAY" else "ACTIVE GATEWAY",
                        fontSize = 11.sp,
                        fontFamily = FontFamily.Monospace,
                        color = if (isConnected) StatusGreen else TextSecondary,
                        fontWeight = FontWeight.Bold
                    )
                    Spacer(modifier = Modifier.height(4.dp))
                    Text(
                        text = status.connectedProfile?.name ?: activeProfile?.name ?: "No Server Selected",
                        fontSize = 16.sp,
                        fontWeight = FontWeight.Bold,
                        color = TextPrimary
                    )
                    val displayServer = status.connectedProfile?.server ?: activeProfile?.server ?: ""
                    val displayPort = status.connectedProfile?.port ?: activeProfile?.port ?: 0
                    Text(
                        text = if (displayServer.isNotEmpty()) "$displayServer:$displayPort" else "Tap to choose a node",
                        fontSize = 12.sp,
                        fontFamily = FontFamily.Monospace,
                        color = if (isConnected) StatusGreen.copy(alpha = 0.8f) else TextTertiary
                    )
                }

                Box(
                    modifier = Modifier
                        .clip(RoundedCornerShape(8.dp))
                        .background(if (isConnected) StatusGreen.copy(alpha = 0.15f) else SurfaceVariantDark)
                        .padding(horizontal = 10.dp, vertical = 6.dp)
                ) {
                    Text(
                        text = if (isConnected) "CLEAN EDGE" else (activeProfile?.type?.name ?: "NONE"),
                        color = if (isConnected) StatusGreen else PrimaryCyan,
                        fontSize = 12.sp,
                        fontFamily = FontFamily.Monospace,
                        fontWeight = FontWeight.Bold
                    )
                }
            }
        }

        // Live Hunting or Status Indicator
        if (isConnecting && status.huntingStatus != null) {
            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .clip(RoundedCornerShape(8.dp))
                    .background(SurfaceDark)
                    .border(1.dp, StatusAmber.copy(alpha = 0.4f), RoundedCornerShape(8.dp))
                    .padding(horizontal = 12.dp, vertical = 8.dp),
                contentAlignment = Alignment.Center
            ) {
                Text(
                    text = status.huntingStatus,
                    color = StatusAmber,
                    fontSize = 12.sp,
                    fontFamily = FontFamily.Monospace,
                    fontWeight = FontWeight.Medium
                )
            }
        } else if (isConnected) {
            Row(
                modifier = Modifier
                    .clip(RoundedCornerShape(8.dp))
                    .background(SurfaceVariantDark)
                    .clickable { onRescanGateway() }
                    .padding(horizontal = 14.dp, vertical = 8.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                Icon(
                    imageVector = Icons.Default.Refresh,
                    contentDescription = "Rescan Gateway",
                    tint = PrimaryCyan,
                    modifier = Modifier.size(16.dp)
                )
                Spacer(modifier = Modifier.width(6.dp))
                Text(
                    text = "FIND FASTER GATEWAY",
                    fontSize = 11.sp,
                    fontFamily = FontFamily.Monospace,
                    fontWeight = FontWeight.Bold,
                    color = PrimaryCyan
                )
            }
        } else {
            Spacer(modifier = Modifier.height(1.dp))
        }

        // Tactile Connect Power Button
        Box(
            modifier = Modifier
                .size(240.dp)
                .scale(pulseScale),
            contentAlignment = Alignment.Center
        ) {
            val outerBorderColor = when {
                isConnected -> StatusGreen
                isConnecting -> StatusAmber
                else -> BorderDark
            }
            val glowBrush = when {
                isConnected -> Brush.radialGradient(listOf(StatusGreenGlow, Color.Transparent))
                isConnecting -> Brush.radialGradient(listOf(StatusAmber.copy(alpha = 0.2f), Color.Transparent))
                else -> Brush.radialGradient(listOf(Color.Transparent, Color.Transparent))
            }

            Box(
                modifier = Modifier
                    .size(240.dp)
                    .clip(CircleShape)
                    .background(glowBrush)
            )

            Box(
                modifier = Modifier
                    .size(190.dp)
                    .clip(CircleShape)
                    .background(SurfaceDark)
                    .border(3.dp, outerBorderColor, CircleShape)
                    .clickable { onToggleConnect() },
                contentAlignment = Alignment.Center
            ) {
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    Icon(
                        imageVector = Icons.Default.PowerSettingsNew,
                        contentDescription = "Toggle Power",
                        modifier = Modifier.size(56.dp),
                        tint = when {
                            isConnected -> StatusGreen
                            isConnecting -> StatusAmber
                            else -> TextSecondary
                        }
                    )
                    Spacer(modifier = Modifier.height(8.dp))
                    Text(
                        text = when {
                            isConnected -> "CONNECTED"
                            isConnecting -> if (status.bestCandidateRtt != null) "HUNTING (${status.bestCandidateRtt}ms)" else "HUNTING IP..."
                            else -> "CONNECT"
                        },
                        fontSize = if (isConnecting) 12.sp else 14.sp,
                        fontFamily = FontFamily.Monospace,
                        fontWeight = FontWeight.Bold,
                        color = when {
                            isConnected -> StatusGreen
                            isConnecting -> StatusAmber
                            else -> TextPrimary
                        },
                        letterSpacing = if (isConnecting) 1.sp else 2.sp
                    )
                }
            }
        }

        // Live Telemetry Stats (Ping, Up, Down)
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .clip(RoundedCornerShape(16.dp))
                .background(SurfaceDark)
                .padding(vertical = 16.dp, horizontal = 12.dp),
            horizontalArrangement = Arrangement.SpaceEvenly,
            verticalAlignment = Alignment.CenterVertically
        ) {
            // Ping
            Column(horizontalAlignment = Alignment.CenterHorizontally) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Icon(
                        imageVector = Icons.Default.Speed,
                        contentDescription = "Ping",
                        modifier = Modifier.size(16.dp),
                        tint = PrimaryCyan
                    )
                    Spacer(modifier = Modifier.width(4.dp))
                    Text(
                        text = "LATENCY",
                        fontSize = 10.sp,
                        fontFamily = FontFamily.Monospace,
                        color = TextSecondary,
                        fontWeight = FontWeight.Bold
                    )
                }
                Spacer(modifier = Modifier.height(4.dp))
                Text(
                    text = if (isConnected && status.pingMs != null) "${status.pingMs} ms" else "-- ms",
                    fontSize = 15.sp,
                    fontFamily = FontFamily.Monospace,
                    fontWeight = FontWeight.Bold,
                    color = if (isConnected) StatusGreen else TextSecondary
                )
            }

            Divider(
                modifier = Modifier
                    .height(32.dp)
                    .width(1.dp),
                color = BorderDark
            )

            // Uplink
            Column(horizontalAlignment = Alignment.CenterHorizontally) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Icon(
                        imageVector = Icons.Default.ArrowUpward,
                        contentDescription = "Upload",
                        modifier = Modifier.size(16.dp),
                        tint = AccentPurple
                    )
                    Spacer(modifier = Modifier.width(4.dp))
                    Text(
                        text = "UPLOAD",
                        fontSize = 10.sp,
                        fontFamily = FontFamily.Monospace,
                        color = TextSecondary,
                        fontWeight = FontWeight.Bold
                    )
                }
                Spacer(modifier = Modifier.height(4.dp))
                Text(
                    text = if (isConnected) "0.0 KB/s" else "--",
                    fontSize = 15.sp,
                    fontFamily = FontFamily.Monospace,
                    fontWeight = FontWeight.Bold,
                    color = TextPrimary
                )
            }

            Divider(
                modifier = Modifier
                    .height(32.dp)
                    .width(1.dp),
                color = BorderDark
            )

            // Downlink
            Column(horizontalAlignment = Alignment.CenterHorizontally) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Icon(
                        imageVector = Icons.Default.ArrowDownward,
                        contentDescription = "Download",
                        modifier = Modifier.size(16.dp),
                        tint = PrimaryCyan
                    )
                    Spacer(modifier = Modifier.width(4.dp))
                    Text(
                        text = "DOWNLOAD",
                        fontSize = 10.sp,
                        fontFamily = FontFamily.Monospace,
                        color = TextSecondary,
                        fontWeight = FontWeight.Bold
                    )
                }
                Spacer(modifier = Modifier.height(4.dp))
                Text(
                    text = if (isConnected) "0.0 KB/s" else "--",
                    fontSize = 15.sp,
                    fontFamily = FontFamily.Monospace,
                    fontWeight = FontWeight.Bold,
                    color = TextPrimary
                )
            }
        }

        Spacer(modifier = Modifier.height(16.dp))
    }
}
