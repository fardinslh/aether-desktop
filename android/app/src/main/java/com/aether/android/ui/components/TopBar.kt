package com.aether.android.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.aether.android.model.VpnState
import com.aether.android.ui.theme.*

@Composable
fun TopBar(
    state: VpnState,
    modifier: Modifier = Modifier
) {
    val (statusText, statusColor, statusBg) = when (state) {
        VpnState.CONNECTED -> Triple("CONNECTED", StatusGreen, StatusGreenGlow)
        VpnState.CONNECTING, VpnState.RECONNECTING -> Triple("CONNECTING", StatusAmber, StatusAmber.copy(alpha = 0.2f))
        VpnState.DISCONNECTED -> Triple("STANDBY", StatusSlate, SurfaceVariantDark)
        VpnState.ERROR -> Triple("ERROR", StatusRed, StatusRed.copy(alpha = 0.2f))
    }

    Row(
        modifier = modifier
            .fillMaxWidth()
            .padding(horizontal = 20.dp, vertical = 14.dp),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(
                text = "AETHER",
                fontSize = 20.sp,
                fontWeight = FontWeight.Black,
                fontFamily = FontFamily.Monospace,
                color = TextPrimary,
                letterSpacing = 2.sp
            )
            Spacer(modifier = Modifier.width(6.dp))
            Box(
                modifier = Modifier
                    .clip(RoundedCornerShape(4.dp))
                    .background(PrimaryCyanGlow)
                    .padding(horizontal = 6.dp, vertical = 2.dp)
            ) {
                Text(
                    text = "MOBILE",
                    color = PrimaryCyan,
                    fontSize = 10.sp,
                    fontFamily = FontFamily.Monospace,
                    fontWeight = FontWeight.Bold
                )
            }
        }

        Box(
            modifier = Modifier
                .clip(RoundedCornerShape(50))
                .background(statusBg)
                .padding(horizontal = 12.dp, vertical = 4.dp)
        ) {
            Text(
                text = statusText,
                color = statusColor,
                fontSize = 11.sp,
                fontFamily = FontFamily.Monospace,
                fontWeight = FontWeight.Bold
            )
        }
    }
}
