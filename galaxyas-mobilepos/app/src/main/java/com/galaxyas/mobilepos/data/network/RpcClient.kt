package com.galaxyas.mobilepos.data.network

import com.galaxyas.mobilepos.data.model.PairResult
import java.io.IOException
import java.util.concurrent.TimeUnit
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.put
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody

/** Jalur yang dipakai untuk menjangkau Server Pusat. */
enum class ConnMode {
    /** Langsung ke IP lokal PC kasir — cepat, tetap jalan saat internet mati. */
    LOCAL,

    /** Lewat relay di VPS — bisa dari mana saja selama PC kasir menyala. */
    ONLINE;

    val label: String get() = if (this == LOCAL) "LOCAL" else "ONLINE"

    companion object {
        fun parse(raw: String?): ConnMode =
            entries.firstOrNull { it.name.equals(raw, ignoreCase = true) } ?: LOCAL
    }
}

/**
 * Alamat Server Pusat yang siap dipakai. `baseUrl` sudah lengkap untuk kedua
 * jalur — LAN `http://ip:8899`, relay `https://host/s/<store_id>` — sehingga
 * rute di bawahnya (`/health`, `/pair`, `/rpc/<cmd>`) sama persis.
 */
data class RemoteConfig(
    val baseUrl: String,
    val token: String,
    val mode: ConnMode,
) {
    fun url(path: String): String = "$baseUrl$path"
}

/** Error dari Server Pusat dengan pesan yang layak tampil ke user. */
open class RpcException(message: String) : Exception(message)

/** 401 — kredensial ditolak; UI harus menawarkan pairing ulang. */
class RpcAuthException(message: String) : RpcException(message)

/**
 * Transport HTTP ke Server Pusat — perilaku meniru desktop lan.rs::call persis:
 * header X-Galaxyas-Token, POST /rpc/<nama>, body kosong = JsonNull, pesan
 * error koneksi VERBATIM sama dengan desktop.
 */
class RpcClient(
    private val remoteProvider: () -> RemoteConfig?,
) {
    companion object {
        const val TOKEN_HEADER = "X-Galaxyas-Token"

        /** Disalin verbatim dari lan.rs supaya UX sama dengan client desktop. */
        const val CONNECT_ERR_MSG =
            "Tidak dapat terhubung ke Server Pusat — periksa koneksi wifi atau apakah PC pusat masih menyala."

        /** Mode ONLINE: yang salah bukan wifi toko, jadi arahannya beda. */
        const val CONNECT_ERR_MSG_ONLINE =
            "Tidak dapat terhubung — periksa koneksi internet HP, atau apakah PC kasir masih menyala."

        const val NOT_PAIRED_MSG = "Belum terhubung ke Server Pusat."
        const val REJECTED_MSG = "Server Pusat menolak permintaan"

        val json = Json {
            ignoreUnknownKeys = true
            explicitNulls = false
            encodeDefaults = true
        }

        private val JSON_MEDIA = "application/json".toMediaType()

        fun connectErrorFor(mode: ConnMode): String =
            if (mode == ConnMode.ONLINE) CONNECT_ERR_MSG_ONLINE else CONNECT_ERR_MSG
    }

    private val base = OkHttpClient.Builder()
        .connectTimeout(15, TimeUnit.SECONDS)
        .readTimeout(15, TimeUnit.SECONDS)
        .writeTimeout(15, TimeUnit.SECONDS)
        .build()

    // Jalur ONLINE menempuh internet seluler + relay + PC kasir; laporan berat
    // bisa lewat 15 detik walau semuanya sehat. Relay sendiri menyerah di 25
    // detik dan membalas 504 dengan pesan yang jelas, jadi batas 30 detik di
    // sini memastikan pesan itu sempat sampai ke user.
    private val online = base.newBuilder()
        .readTimeout(30, TimeUnit.SECONDS)
        .writeTimeout(30, TimeUnit.SECONDS)
        .build()

    private fun clientFor(mode: ConnMode) = if (mode == ConnMode.ONLINE) online else base

    /** POST /rpc/<name> ke server aktif; hasil mentah JsonElement. */
    suspend fun call(name: String, args: JsonElement): JsonElement {
        val remote = remoteProvider() ?: throw RpcException(NOT_PAIRED_MSG)
        return callTo(remote, name, args)
    }

    suspend fun callTo(remote: RemoteConfig, name: String, args: JsonElement): JsonElement =
        withContext(Dispatchers.IO) {
            val request = Request.Builder()
                .url(remote.url("/rpc/$name"))
                .header(TOKEN_HEADER, remote.token)
                .post(json.encodeToString(JsonElement.serializer(), args).toRequestBody(JSON_MEDIA))
                .build()
            val body = execute(remote, request)
            // Body kosong dianggap null (command yang me-return unit).
            if (body.isBlank()) JsonNull
            else try {
                json.parseToJsonElement(body)
            } catch (e: Exception) {
                throw RpcException("respons Server Pusat tidak valid: ${e.message}")
            }
        }

    /**
     * Tukar kode pairing 6 karakter dengan token milik HP ini. Sesudah ini kode
     * pairing tidak dipakai lagi — token panjang yang dipakai tiap permintaan,
     * dan pemilik toko bisa mencabutnya per HP dari layar Pengaturan di PC.
     */
    suspend fun pair(remote: RemoteConfig, code: String, deviceName: String): PairResult =
        withContext(Dispatchers.IO) {
            val payload = buildJsonObject {
                put("code", code)
                put("device_name", deviceName)
            }
            val request = Request.Builder()
                .url(remote.url("/pair"))
                .post(json.encodeToString(JsonElement.serializer(), payload).toRequestBody(JSON_MEDIA))
                .build()
            val body = execute(remote, request)
            try {
                json.decodeFromString(PairResult.serializer(), body)
            } catch (e: Exception) {
                throw RpcException("respons Server Pusat tidak valid: ${e.message}")
            }
        }

    /**
     * GET /health — dipakai ConnectionWatcher dan tombol "Coba lagi". Di jalur
     * ONLINE ini dijawab relay sendiri (tanpa membangunkan PC kasir), jadi murah
     * dipanggil berkala.
     */
    suspend fun healthCheck(remote: RemoteConfig): Unit =
        withContext(Dispatchers.IO) {
            val request = Request.Builder()
                .url(remote.url("/health"))
                .header(TOKEN_HEADER, remote.token)
                .get()
                .build()
            execute(remote, request)
        }

    /** Kirim satu permintaan dan kembalikan body-nya, atau lempar pesan siap tampil. */
    private fun execute(remote: RemoteConfig, request: Request): String {
        val response = try {
            clientFor(remote.mode).newCall(request).execute()
        } catch (_: IOException) {
            throw RpcException(connectErrorFor(remote.mode))
        }
        return response.use { resp ->
            val body = try {
                resp.body?.string() ?: ""
            } catch (_: IOException) {
                throw RpcException(connectErrorFor(remote.mode))
            }
            if (!resp.isSuccessful) {
                // 503/504 dari relay sudah membawa pesan Indonesia yang jelas
                // ("PC kasir sedang mati…"), jadi diteruskan apa adanya.
                val msg = parseErrorMessage(body) ?: REJECTED_MSG
                if (resp.code == 401) throw RpcAuthException(msg) else throw RpcException(msg)
            }
            body
        }
    }

    private fun parseErrorMessage(body: String): String? = try {
        json.parseToJsonElement(body).jsonObject["error"]?.jsonPrimitive?.content
    } catch (_: Exception) {
        null
    }
}
