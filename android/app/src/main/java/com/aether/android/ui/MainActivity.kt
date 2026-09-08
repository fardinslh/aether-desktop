package com.aether.android.ui

import android.app.Activity
import android.net.VpnService
import android.os.Bundle
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.lifecycle.lifecycleScope
import com.aether.android.AetherApplication
import com.aether.android.model.AppInfo
import com.aether.android.model.VpnState
import com.aether.android.repository.AppListRepository
import com.aether.android.service.AetherVpnService
import com.aether.android.ui.components.BottomNavBar
import com.aether.android.ui.components.Screen
import com.aether.android.ui.components.TopBar
import com.aether.android.ui.screens.HomeScreen
import com.aether.android.ui.screens.ServersScreen
import com.aether.android.ui.screens.SettingsScreen
import com.aether.android.ui.screens.SplitTunnelScreen
import com.aether.android.ui.theme.AetherTheme
import com.aether.android.ui.theme.BackgroundDark
import kotlinx.coroutines.launch

class MainActivity : ComponentActivity() {

    private val vpnPermissionLauncher =
        registerForActivityResult(ActivityResultContracts.StartActivityForResult()) { result ->
            if (result.resultCode == Activity.RESULT_OK) {
                AetherVpnService.start(this)
            } else {
                Toast.makeText(this, "VPN permission is required to route traffic", Toast.LENGTH_SHORT).show()
            }
        }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val app = AetherApplication.instance
        val profileRepo = app.profileRepository
        val settingsRepo = app.settingsRepository
        val appListRepo = AppListRepository(this)

        setContent {
            AetherTheme {
                val vpnStatus by AetherVpnService.vpnStatus.collectAsState()
                val profiles by profileRepo.profiles.collectAsState()
                val settings by settingsRepo.settings.collectAsState()

                var currentRoute by remember { mutableStateOf(Screen.Home.route) }
                var installedApps by remember { mutableStateOf<List<AppInfo>>(emptyList()) }

                // Load installed apps once
                LaunchedEffect(Unit) {
                    installedApps = appListRepo.getInstalledApps()
                }

                // Selected profile lookup
                val activeProfile = remember(profiles, settings.selectedProfileId) {
                    profiles.find { it.id == settings.selectedProfileId } ?: profiles.firstOrNull()
                }

                Scaffold(
                    topBar = {
                        TopBar(state = vpnStatus.state)
                    },
                    bottomBar = {
                        BottomNavBar(
                            currentRoute = currentRoute,
                            onNavigate = { currentRoute = it }
                        )
                    },
                    containerColor = BackgroundDark
                ) { innerPadding ->
                    Box(
                        modifier = Modifier
                            .fillMaxSize()
                            .padding(innerPadding)
                    ) {
                        when (currentRoute) {
                            Screen.Home.route -> {
                                HomeScreen(
                                    status = vpnStatus,
                                    activeProfile = activeProfile,
                                    onToggleConnect = {
                                        if (vpnStatus.state == VpnState.CONNECTED || vpnStatus.state == VpnState.CONNECTING) {
                                            AetherVpnService.stop(this@MainActivity)
                                        } else {
                                            val prepareIntent = VpnService.prepare(this@MainActivity)
                                            if (prepareIntent != null) {
                                                vpnPermissionLauncher.launch(prepareIntent)
                                            } else {
                                                AetherVpnService.start(this@MainActivity)
                                            }
                                        }
                                    },
                                    onNavigateToServers = {
                                        currentRoute = Screen.Servers.route
                                    }
                                )
                            }
                            Screen.Servers.route -> {
                                ServersScreen(
                                    profiles = profiles,
                                    selectedProfileId = activeProfile?.id,
                                    onSelectProfile = { id ->
                                        settingsRepo.selectProfile(id)
                                        if (vpnStatus.state == VpnState.CONNECTED) {
                                            // Reconnect with new profile
                                            AetherVpnService.start(this@MainActivity, id)
                                        }
                                    },
                                    onAddProfile = { profile ->
                                        profileRepo.addProfile(profile)
                                    },
                                    onDeleteProfile = { id ->
                                        profileRepo.deleteProfile(id)
                                    },
                                    onImportSubscription = { url ->
                                        lifecycleScope.launch {
                                            val res = profileRepo.importSubscription(url)
                                            res.onSuccess { count ->
                                                Toast.makeText(this@MainActivity, "Imported $count nodes", Toast.LENGTH_SHORT).show()
                                            }.onFailure { err ->
                                                Toast.makeText(this@MainActivity, "Failed: ${err.message}", Toast.LENGTH_SHORT).show()
                                            }
                                        }
                                    }
                                )
                            }
                            Screen.SplitTunnel.route -> {
                                SplitTunnelScreen(
                                    installedApps = installedApps,
                                    selectedMode = settings.splitTunnelMode,
                                    selectedPackages = settings.selectedPackages,
                                    onModeChange = { mode ->
                                        settingsRepo.setSplitTunnelMode(mode)
                                    },
                                    onToggleApp = { pkg ->
                                        settingsRepo.toggleAppSelection(pkg)
                                    }
                                )
                            }
                            Screen.Settings.route -> {
                                SettingsScreen(
                                    settings = settings,
                                    onUpdateSettings = { transform ->
                                        settingsRepo.updateSettings(transform)
                                    }
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}
