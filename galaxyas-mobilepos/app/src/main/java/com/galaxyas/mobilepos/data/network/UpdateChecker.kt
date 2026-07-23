package com.galaxyas.mobilepos.data.network

import java.util.concurrent.TimeUnit
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import okhttp3.OkHttpClient
import okhttp3.Request

@Serializable
data class MobileRelease(
    val version: String,
    val apk_url: String,
    val notes: String = "",
)

/**
 * Cek versi terbaru dari aset rilis publik (galaxyas-pos-releases). Android
 * tidak punya updater bawaan seperti desktop, jadi app hanya memberi tahu +
 * membuka link APK; pemasangan tetap manual oleh user.
 */
object UpdateChecker {
    const val LATEST_URL =
        "https://github.com/savinsaka/galaxyas-pos-releases/releases/latest/download/mobile-latest.json"

    private val http = OkHttpClient.Builder()
        .connectTimeout(10, TimeUnit.SECONDS)
        .readTimeout(10, TimeUnit.SECONDS)
        .build()

    private val json = Json { ignoreUnknownKeys = true }

    /** Kembalikan rilis bila LEBIH BARU dari `current`; null bila sudah terbaru/gagal. */
    suspend fun check(current: String): MobileRelease? = withContext(Dispatchers.IO) {
        runCatching {
            val req = Request.Builder().url(LATEST_URL).get().build()
            http.newCall(req).execute().use { resp ->
                if (!resp.isSuccessful) return@use null
                val body = resp.body?.string().orEmpty()
                val rel = json.decodeFromString<MobileRelease>(body)
                if (isNewer(rel.version, current)) rel else null
            }
        }.getOrNull()
    }

    /** Perbandingan semver sederhana (x.y.z), toleran format tak lengkap. */
    fun isNewer(candidate: String, current: String): Boolean {
        fun parts(v: String) = v.trim().removePrefix("v").split(".")
            .map { it.takeWhile { c -> c.isDigit() }.toIntOrNull() ?: 0 }
        val a = parts(candidate)
        val b = parts(current)
        for (i in 0 until maxOf(a.size, b.size)) {
            val x = a.getOrElse(i) { 0 }
            val y = b.getOrElse(i) { 0 }
            if (x != y) return x > y
        }
        return false
    }
}
