package com.aether.android.repository

import android.content.Context
import android.content.Intent
import android.net.Uri
import android.util.Log
import com.google.gson.Gson
import com.google.gson.annotations.SerializedName
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.withContext
import okhttp3.OkHttpClient
import okhttp3.Request
import java.util.concurrent.TimeUnit

data class GithubAsset(
    val name: String,
    @SerializedName("browser_download_url") val downloadUrl: String,
    val size: Long
)

data class GithubRelease(
    @SerializedName("tag_name") val tagName: String,
    val name: String?,
    @SerializedName("html_url") val htmlUrl: String?,
    val body: String?,
    val assets: List<GithubAsset>?
)

data class AppUpdateInfo(
    val hasUpdate: Boolean,
    val currentVersion: String,
    val latestVersion: String,
    val releaseNotes: String?,
    val releaseUrl: String?,
    val downloadUrl: String?,
    val aetherCoreVersion: String? = null
)

class UpdateRepository(private val context: Context) {

    companion object {
        private const val TAG = "UpdateRepository"
        private const val APP_REPO_URL = "https://api.github.com/repos/fardinslh/aether-desktop/releases/latest"
        private const val AETHER_CORE_REPO_URL = "https://api.github.com/repos/CluvexStudio/Aether/releases/latest"
        const val CURRENT_APP_VERSION = "0.1.3"
    }

    private val client = OkHttpClient.Builder()
        .connectTimeout(15, TimeUnit.SECONDS)
        .readTimeout(15, TimeUnit.SECONDS)
        .build()

    private val gson = Gson()

    private val _updateInfo = MutableStateFlow<AppUpdateInfo?>(null)
    val updateInfo: StateFlow<AppUpdateInfo?> = _updateInfo.asStateFlow()

    private val _isChecking = MutableStateFlow(false)
    val isChecking: StateFlow<Boolean> = _isChecking.asStateFlow()

    suspend fun checkForUpdates(currentVersion: String = CURRENT_APP_VERSION): AppUpdateInfo? = withContext(Dispatchers.IO) {
        _isChecking.value = true
        try {
            var coreLatest = "v2.3.0"
            try {
                val coreReq = Request.Builder()
                    .url(AETHER_CORE_REPO_URL)
                    .header("User-Agent", "Aether-Android")
                    .header("Accept", "application/vnd.github.v3+json")
                    .build()
                client.newCall(coreReq).execute().use { resp ->
                    if (resp.isSuccessful) {
                        val body = resp.body?.string()
                        if (body != null) {
                            val rel = gson.fromJson(body, GithubRelease::class.java)
                            coreLatest = rel.tagName
                        }
                    }
                }
            } catch (e: Exception) {
                Log.w(TAG, "Failed to check Aether Core release: ${e.message}")
            }

            val appReq = Request.Builder()
                .url(APP_REPO_URL)
                .header("User-Agent", "Aether-Android")
                .header("Accept", "application/vnd.github.v3+json")
                .build()

            client.newCall(appReq).execute().use { resp ->
                if (!resp.isSuccessful) {
                    Log.w(TAG, "GitHub API returned code ${resp.code}")
                    return@withContext null
                }
                val body = resp.body?.string() ?: return@withContext null
                val release = gson.fromJson(body, GithubRelease::class.java) ?: return@withContext null

                val latestTag = release.tagName.trim()
                val latestClean = latestTag.removePrefix("v").removePrefix("V")
                val currentClean = currentVersion.trim().removePrefix("v").removePrefix("V")

                val isNewer = isVersionNewer(latestClean, currentClean)

                val apkAsset = release.assets?.firstOrNull { it.name.endsWith(".apk") }
                val downloadUrl = apkAsset?.downloadUrl ?: release.htmlUrl

                val info = AppUpdateInfo(
                    hasUpdate = isNewer,
                    currentVersion = currentClean,
                    latestVersion = latestClean,
                    releaseNotes = release.body,
                    releaseUrl = release.htmlUrl,
                    downloadUrl = downloadUrl,
                    aetherCoreVersion = coreLatest
                )

                _updateInfo.value = info
                info
            }
        } catch (e: Exception) {
            Log.e(TAG, "Error checking for updates: ${e.message}", e)
            null
        } finally {
            _isChecking.value = false
        }
    }

    private fun isVersionNewer(latest: String, current: String): Boolean {
        try {
            val latestParts = latest.split('.').map { it.filter { c -> c.isDigit() }.toIntOrNull() ?: 0 }
            val currentParts = current.split('.').map { it.filter { c -> c.isDigit() }.toIntOrNull() ?: 0 }

            val maxLen = maxOf(latestParts.size, currentParts.size)
            for (i in 0 until maxLen) {
                val l = latestParts.getOrElse(i) { 0 }
                val c = currentParts.getOrElse(i) { 0 }
                if (l > c) return true
                if (l < c) return false
            }
            return false
        } catch (e: Exception) {
            return false
        }
    }

    fun openDownloadUrl(targetUrl: String?) {
        val url = targetUrl ?: return
        try {
            val intent = Intent(Intent.ACTION_VIEW, Uri.parse(url)).apply {
                addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            }
            context.startActivity(intent)
        } catch (e: Exception) {
            Log.e(TAG, "Failed to launch download intent for $url", e)
        }
    }
}
