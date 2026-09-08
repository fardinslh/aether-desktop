package com.aether.android.service

import android.content.Intent
import android.net.VpnService
import android.os.Build
import android.service.quicksettings.Tile
import android.service.quicksettings.TileService
import androidx.annotation.RequiresApi
import com.aether.android.model.VpnState
import com.aether.android.ui.MainActivity
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.launch

@RequiresApi(Build.VERSION_CODES.N)
class AetherTileService : TileService() {

    private val serviceScope = CoroutineScope(Dispatchers.Main)
    private var stateObserverJob: Job? = null

    override fun onStartListening() {
        super.onStartListening()
        stateObserverJob?.cancel()
        stateObserverJob = serviceScope.launch {
            AetherVpnService.vpnStatus.collectLatest { status ->
                updateTileState(status.state)
            }
        }
    }

    override fun onStopListening() {
        stateObserverJob?.cancel()
        super.onStopListening()
    }

    override fun onClick() {
        super.onClick()
        val current = AetherVpnService.vpnStatus.value.state
        if (current == VpnState.CONNECTED || current == VpnState.CONNECTING) {
            AetherVpnService.stop(this)
        } else {
            val prepareIntent = VpnService.prepare(this)
            if (prepareIntent != null) {
                // Requires user system dialog authorization -> open app
                val appIntent = Intent(this, MainActivity::class.java).apply {
                    flags = Intent.FLAG_ACTIVITY_NEW_TASK
                }
                startActivityAndCollapse(appIntent)
            } else {
                AetherVpnService.start(this)
            }
        }
    }

    private fun updateTileState(state: VpnState) {
        val tile = qsTile ?: return
        when (state) {
            VpnState.CONNECTED -> {
                tile.state = Tile.STATE_ACTIVE
                tile.subtitle = "Connected"
            }
            VpnState.CONNECTING, VpnState.RECONNECTING -> {
                tile.state = Tile.STATE_ACTIVE
                tile.subtitle = "Connecting…"
            }
            VpnState.DISCONNECTED, VpnState.ERROR -> {
                tile.state = Tile.STATE_INACTIVE
                tile.subtitle = "Disconnected"
            }
        }
        tile.updateTile()
    }
}
