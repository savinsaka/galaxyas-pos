package com.galaxyas.mobilepos.data

import com.galaxyas.mobilepos.data.model.PairingPayload
import kotlinx.serialization.json.Json

/** Versi format QR tertinggi yang dimengerti app ini (desktop: PAIRING_PAYLOAD_VERSION). */
const val PAIRING_QR_VERSION = 1

/**
 * Hasil membaca QR di layar pairing. Kamera yang sama juga dipakai untuk barcode
 * barang, jadi "bukan QR pairing" adalah hasil yang wajar — bukan error yang
 * perlu ditakuti, cukup diberitahu supaya kasir tidak bingung.
 */
sealed interface PairingQrResult {
    data class Ok(val payload: PairingPayload) : PairingQrResult

    /** Bukan QR pairing (mis. barcode barang atau QR dari aplikasi lain). */
    data object NotPairingQr : PairingQrResult

    /** QR dibuat desktop yang lebih baru — app HP perlu di-update dulu. */
    data class TooNew(val version: Int) : PairingQrResult
}

private val json = Json { ignoreUnknownKeys = true; isLenient = true }

/**
 * Baca isi QR pairing. Sengaja tidak melempar exception untuk isi yang tidak
 * dikenal: layar scanner memanggil ini untuk SETIAP kode yang terbaca kamera.
 */
fun parsePairingQr(raw: String): PairingQrResult {
    val text = raw.trim()
    if (!text.startsWith("{")) return PairingQrResult.NotPairingQr

    val payload = runCatching {
        json.decodeFromString(PairingPayload.serializer(), text)
    }.getOrNull() ?: return PairingQrResult.NotPairingQr

    // Kode pairing adalah penanda paling khas; JSON lain yang kebetulan terbaca
    // hampir pasti tidak punya field ini.
    if (payload.code.isBlank()) return PairingQrResult.NotPairingQr
    if (payload.v > PAIRING_QR_VERSION) return PairingQrResult.TooNew(payload.v)
    // Tanpa salah satu alamat, tidak ada yang bisa dihubungi.
    if (payload.host.isBlank() && (payload.relay.isBlank() || payload.store_id.isBlank())) {
        return PairingQrResult.NotPairingQr
    }
    return PairingQrResult.Ok(payload)
}
