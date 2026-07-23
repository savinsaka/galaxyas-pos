package com.galaxyas.mobilepos.data.network

import java.io.IOException
import java.util.concurrent.TimeUnit
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody

/** Konfigurasi Server Pusat aktif (padanan RemoteConfig di desktop lan.rs). */
data class RemoteConfig(
    val host: String,
    val port: Int,
    val token: String,
) {
    val baseUrl: String get() = "http://$host:$port"
}

/** Error dari Server Pusat dengan pesan yang layak tampil ke user. */
open class RpcException(message: String) : Exception(message)

/** 401 — token pairing ditolak; UI harus menawarkan pairing ulang. */
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
        const val CONNECT_ERR_MSG =
            "Tidak dapat terhubung ke Server Pusat — periksa koneksi wifi atau apakah PC pusat masih menyala."
        const val NOT_PAIRED_MSG = "Belum terhubung ke Server Pusat."
        const val REJECTED_MSG = "Server Pusat menolak permintaan"

        val json = Json {
            ignoreUnknownKeys = true
            explicitNulls = false
            encodeDefaults = true
        }

        private val JSON_MEDIA = "application/json".toMediaType()
    }

    private val http = OkHttpClient.Builder()
        .connectTimeout(15, TimeUnit.SECONDS)
        .readTimeout(15, TimeUnit.SECONDS)
        .writeTimeout(15, TimeUnit.SECONDS)
        .build()

    /** POST /rpc/<name> ke server aktif; hasil mentah JsonElement. */
    suspend fun call(name: String, args: JsonElement): JsonElement {
        val remote = remoteProvider() ?: throw RpcException(NOT_PAIRED_MSG)
        return callTo(remote, name, args)
    }

    suspend fun callTo(remote: RemoteConfig, name: String, args: JsonElement): JsonElement =
        withContext(Dispatchers.IO) {
            val request = Request.Builder()
                .url("${remote.baseUrl}/rpc/$name")
                .header(TOKEN_HEADER, remote.token)
                .post(json.encodeToString(JsonElement.serializer(), args).toRequestBody(JSON_MEDIA))
                .build()

            val response = try {
                http.newCall(request).execute()
            } catch (_: IOException) {
                throw RpcException(CONNECT_ERR_MSG)
            }

            response.use { resp ->
                val body = try {
                    resp.body?.string() ?: ""
                } catch (_: IOException) {
                    throw RpcException(CONNECT_ERR_MSG)
                }
                if (!resp.isSuccessful) {
                    val msg = parseErrorMessage(body) ?: REJECTED_MSG
                    if (resp.code == 401) throw RpcAuthException(msg) else throw RpcException(msg)
                }
                // Body kosong dianggap null (command yang me-return unit).
                if (body.isBlank()) JsonNull
                else try {
                    json.parseToJsonElement(body)
                } catch (e: Exception) {
                    throw RpcException("respons Server Pusat tidak valid: ${e.message}")
                }
            }
        }

    /**
     * GET /health dengan token — validasi pairing SEBELUM server disimpan
     * (alur "+ Tambah Server"), padanan lan.rs::health_check.
     */
    suspend fun healthCheck(host: String, port: Int, token: String): Unit =
        withContext(Dispatchers.IO) {
            val request = Request.Builder()
                .url("http://$host:$port/health")
                .header(TOKEN_HEADER, token)
                .get()
                .build()
            val response = try {
                http.newCall(request).execute()
            } catch (_: IOException) {
                throw RpcException(CONNECT_ERR_MSG)
            }
            response.use { resp ->
                if (resp.code == 401) throw RpcAuthException("kode pairing salah")
                if (!resp.isSuccessful) throw RpcException(CONNECT_ERR_MSG)
            }
        }

    private fun parseErrorMessage(body: String): String? = try {
        json.parseToJsonElement(body).jsonObject["error"]?.jsonPrimitive?.content
    } catch (_: Exception) {
        null
    }
}
