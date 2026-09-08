package com.aether.android.repository

import android.content.Context
import android.content.SharedPreferences
import com.aether.android.core.V2RayParser
import com.aether.android.model.Profile
import com.aether.android.model.ProfileType
import com.google.gson.Gson
import com.google.gson.reflect.TypeToken
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.withContext
import okhttp3.OkHttpClient
import okhttp3.Request
import java.util.concurrent.TimeUnit

class ProfileRepository(context: Context) {

    private val prefs: SharedPreferences =
        context.getSharedPreferences("aether_profiles_pref", Context.MODE_PRIVATE)
    private val gson = Gson()
    private val httpClient = OkHttpClient.Builder()
        .connectTimeout(15, TimeUnit.SECONDS)
        .readTimeout(20, TimeUnit.SECONDS)
        .build()

    private val _profiles = MutableStateFlow<List<Profile>>(emptyList())
    val profiles: StateFlow<List<Profile>> = _profiles.asStateFlow()

    init {
        loadProfiles()
    }

    private fun loadProfiles() {
        val json = prefs.getString("profiles_json", null)
        if (json != null) {
            val type = object : TypeToken<List<Profile>>() {}.type
            val list: List<Profile> = gson.fromJson(json, type) ?: emptyList()
            _profiles.value = list
        } else {
            // Seed factory default Cloudflare WARP Edge Node
            val defaultWarp = Profile(
                id = "factory_warp",
                name = "Cloudflare WARP Edge",
                type = ProfileType.WARP,
                server = "162.159.192.1",
                port = 2408,
                publicKey = "bmXOC+F1FxEMF9dyiK2H5/1SUtzH0JuVo51h2wPfgyo="
            )
            val initial = listOf(defaultWarp)
            saveProfilesInternal(initial)
            _profiles.value = initial
        }
    }

    private fun saveProfilesInternal(list: List<Profile>) {
        val json = gson.toJson(list)
        prefs.edit().putString("profiles_json", json).apply()
        _profiles.value = list
    }

    fun addProfile(profile: Profile) {
        val updated = _profiles.value.toMutableList().apply { add(0, profile) }
        saveProfilesInternal(updated)
    }

    fun updateProfile(profile: Profile) {
        val updated = _profiles.value.map { if (it.id == profile.id) profile else it }
        saveProfilesInternal(updated)
    }

    fun deleteProfile(id: String) {
        val updated = _profiles.value.filter { it.id != id }
        saveProfilesInternal(updated)
    }

    fun getProfile(id: String): Profile? {
        return _profiles.value.find { it.id == id }
    }

    suspend fun importSubscription(url: String): Result<Int> = withContext(Dispatchers.IO) {
        try {
            val request = Request.Builder()
                .url(url)
                .header("User-Agent", "Aether/1.0.0 (Android)")
                .build()
            val response = httpClient.newCall(request).execute()
            if (!response.isSuccessful) {
                return@withContext Result.failure(Exception("HTTP error ${response.code}"))
            }
            val body = response.body?.string() ?: ""
            val imported = V2RayParser.parseSubscription(body)
            if (imported.isEmpty()) {
                return@withContext Result.failure(Exception("No supported configs found in subscription"))
            }

            val current = _profiles.value.toMutableList()
            for (p in imported) {
                if (current.none { it.server == p.server && it.port == p.port && it.uuid == p.uuid }) {
                    current.add(p)
                }
            }
            saveProfilesInternal(current)
            Result.success(imported.size)
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
}
