package com.aether.android.repository

import android.content.Context
import android.content.SharedPreferences
import com.aether.android.model.AppSettings
import com.aether.android.model.SplitTunnelMode
import com.google.gson.Gson
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

class SettingsRepository(context: Context) {

    private val prefs: SharedPreferences =
        context.getSharedPreferences("aether_settings_pref", Context.MODE_PRIVATE)
    private val gson = Gson()

    private val _settings = MutableStateFlow(AppSettings())
    val settings: StateFlow<AppSettings> = _settings.asStateFlow()

    init {
        loadSettings()
    }

    private fun loadSettings() {
        val json = prefs.getString("app_settings_json", null)
        if (json != null) {
            val s: AppSettings = gson.fromJson(json, AppSettings::class.java) ?: AppSettings()
            _settings.value = s
        } else {
            val initial = AppSettings()
            saveSettingsInternal(initial)
            _settings.value = initial
        }
    }

    private fun saveSettingsInternal(settings: AppSettings) {
        val json = gson.toJson(settings)
        prefs.edit().putString("app_settings_json", json).apply()
        _settings.value = settings
    }

    fun updateSettings(transform: (AppSettings) -> AppSettings) {
        val updated = transform(_settings.value)
        saveSettingsInternal(updated)
    }

    fun setSplitTunnelMode(mode: SplitTunnelMode) {
        updateSettings { it.copy(splitTunnelMode = mode) }
    }

    fun toggleAppSelection(packageName: String) {
        updateSettings { current ->
            val set = current.selectedPackages.toMutableSet()
            if (set.contains(packageName)) {
                set.remove(packageName)
            } else {
                set.add(packageName)
            }
            current.copy(selectedPackages = set)
        }
    }

    fun selectProfile(id: String) {
        updateSettings { it.copy(selectedProfileId = id) }
    }
}
