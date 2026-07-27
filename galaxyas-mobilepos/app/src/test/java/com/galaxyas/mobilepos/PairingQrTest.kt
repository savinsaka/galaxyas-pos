package com.galaxyas.mobilepos

import com.galaxyas.mobilepos.data.PairingQrResult
import com.galaxyas.mobilepos.data.parsePairingQr
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Parser QR pairing. Kamera yang sama juga membaca barcode barang, jadi parser
 * ini dipanggil untuk apa pun yang kebetulan masuk frame — salah baca tidak
 * boleh membuat app crash atau, lebih buruk, mengisi alamat server dengan
 * sampah yang lalu disimpan kasir.
 */
class PairingQrTest {

    /** Bentuk yang benar-benar dihasilkan desktop (lan.rs::PairingPayload). */
    private val dariDesktop = """
        {"v":1,"name":"GALAXYAS Toko 1","host":"192.168.18.11","port":8899,
         "relay":"relay.jjapps.net","store_id":"ed2a445ded3d12c68a8b9c7a3e58a18e",
         "code":"A1B2C3"}
    """.trimIndent()

    @Test
    fun `qr lengkap terbaca utuh`() {
        val r = parsePairingQr(dariDesktop)
        assertTrue(r is PairingQrResult.Ok)
        val p = (r as PairingQrResult.Ok).payload
        assertEquals("GALAXYAS Toko 1", p.name)
        assertEquals("192.168.18.11", p.host)
        assertEquals(8899, p.port)
        assertEquals("relay.jjapps.net", p.relay)
        assertEquals("ed2a445ded3d12c68a8b9c7a3e58a18e", p.store_id)
        assertEquals("A1B2C3", p.code)
    }

    @Test
    fun `qr hanya LAN tetap sah`() {
        val raw = """{"v":1,"name":"Toko","host":"192.168.1.5","port":8899,"relay":"","store_id":"","code":"ZZ9988"}"""
        val r = parsePairingQr(raw)
        assertTrue(r is PairingQrResult.Ok)
        assertEquals("", (r as PairingQrResult.Ok).payload.relay)
    }

    @Test
    fun `qr hanya relay tetap sah`() {
        val raw = """{"v":1,"name":"Toko","host":"","relay":"relay.jjapps.net","store_id":"abc","code":"ZZ9988"}"""
        assertTrue(parsePairingQr(raw) is PairingQrResult.Ok)
    }

    @Test
    fun `barcode barang biasa bukan qr pairing`() {
        assertEquals(PairingQrResult.NotPairingQr, parsePairingQr("8991002101234"))
        assertEquals(PairingQrResult.NotPairingQr, parsePairingQr("Aqua 600ml"))
        assertEquals(PairingQrResult.NotPairingQr, parsePairingQr(""))
    }

    @Test
    fun `qr aplikasi lain diabaikan, bukan crash`() {
        assertEquals(PairingQrResult.NotPairingQr, parsePairingQr("https://wa.me/628123"))
        assertEquals(PairingQrResult.NotPairingQr, parsePairingQr("""{"foo":"bar"}"""))
        assertEquals(PairingQrResult.NotPairingQr, parsePairingQr("{ rusak json"))
    }

    /** Tanpa kode pairing, QR itu tidak ada gunanya — jangan diterima separuh. */
    @Test
    fun `qr tanpa kode pairing ditolak`() {
        val raw = """{"v":1,"name":"Toko","host":"192.168.1.5","port":8899,"code":""}"""
        assertEquals(PairingQrResult.NotPairingQr, parsePairingQr(raw))
    }

    /** Ada kode tapi tidak ada satu pun alamat = tidak ada yang bisa dihubungi. */
    @Test
    fun `qr tanpa alamat apa pun ditolak`() {
        val raw = """{"v":1,"name":"Toko","host":"","relay":"","store_id":"","code":"A1B2C3"}"""
        assertEquals(PairingQrResult.NotPairingQr, parsePairingQr(raw))

        // Relay diisi tapi store_id kosong = alamat relay belum lengkap.
        val setengah = """{"v":1,"host":"","relay":"relay.jjapps.net","store_id":"","code":"A1B2C3"}"""
        assertEquals(PairingQrResult.NotPairingQr, parsePairingQr(setengah))
    }

    /** Desktop lebih baru: beri tahu update, jangan diam-diam salah tafsir. */
    @Test
    fun `qr versi lebih baru minta update app`() {
        val raw = """{"v":9,"name":"Toko","host":"192.168.1.5","port":8899,"code":"A1B2C3"}"""
        val r = parsePairingQr(raw)
        assertTrue(r is PairingQrResult.TooNew)
        assertEquals(9, (r as PairingQrResult.TooNew).version)
    }

    /** Field baru dari desktop versi depan tidak boleh bikin gagal parse. */
    @Test
    fun `field tak dikenal diabaikan`() {
        val raw = """{"v":1,"name":"Toko","host":"192.168.1.5","port":8899,"code":"A1B2C3","fitur_baru":123}"""
        assertTrue(parsePairingQr(raw) is PairingQrResult.Ok)
    }

    @Test
    fun `spasi di sekitar isi qr dimaafkan`() {
        val r = parsePairingQr("  $dariDesktop  ")
        assertTrue(r is PairingQrResult.Ok)
    }
}
