package com.galaxyas.mobilepos

import com.galaxyas.mobilepos.data.SavedServer
import com.galaxyas.mobilepos.data.network.ConnMode
import kotlinx.serialization.json.Json
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Entri server dua-jalur. Yang paling penting dijaga: HP yang sudah terpasang
 * (format lama `host/port/token`) tidak boleh diminta pairing ulang setelah
 * update — kasir bisa kehilangan akses di tengah jam kerja.
 */
class SavedServerTest {
    private val json = Json { ignoreUnknownKeys = true; encodeDefaults = true }

    @Test
    fun `entri versi lama dibaca dan dipindah ke field baru`() {
        val lama = """{"id":"s1","name":"Kasir Pusat","host":"192.168.1.10","port":8899,"token":"A1B2C3"}"""
        val server = json.decodeFromString(SavedServer.serializer(), lama).migrated()

        assertEquals("192.168.1.10", server.lan_host)
        assertEquals(8899, server.lan_port)
        assertEquals("A1B2C3", server.device_token)
        assertTrue(server.hasLan)
        assertFalse("belum pernah diatur online", server.hasOnline)
        // Field lama dikosongkan supaya migrasi tidak jalan dua kali.
        assertNull(server.host)
        assertNull(server.port)
        assertNull(server.token)
    }

    @Test
    fun `port bukan bawaan pada entri lama ikut terbawa`() {
        val lama = """{"id":"s1","name":"X","host":"10.0.0.5","port":9100,"token":"ZZ1122"}"""
        val server = json.decodeFromString(SavedServer.serializer(), lama).migrated()
        assertEquals(9100, server.lan_port)
    }

    @Test
    fun `migrasi bersifat idempoten`() {
        val lama = """{"id":"s1","name":"X","host":"10.0.0.5","port":8899,"token":"ZZ1122"}"""
        val sekali = json.decodeFromString(SavedServer.serializer(), lama).migrated()
        assertEquals(sekali, sekali.migrated())
    }

    @Test
    fun `base url LOCAL dan ONLINE dibentuk sesuai rute masing-masing`() {
        val server = SavedServer(
            id = "s1",
            name = "Kasir Pusat",
            lan_host = "192.168.1.10",
            lan_port = 8899,
            relay_url = "relay.jjapps.net",
            store_id = "abc123",
            device_token = "t".repeat(64),
        )
        assertEquals("http://192.168.1.10:8899", server.toRemote(ConnMode.LOCAL)!!.baseUrl)
        assertEquals("https://relay.jjapps.net/s/abc123", server.toRemote(ConnMode.ONLINE)!!.baseUrl)
    }

    @Test
    fun `url relay diterima dalam berbagai bentuk ketikan`() {
        fun online(url: String) = SavedServer(
            id = "s", name = "n", relay_url = url, store_id = "abc", device_token = "t",
        ).toRemote(ConnMode.ONLINE)!!.baseUrl

        assertEquals("https://relay.jjapps.net/s/abc", online("relay.jjapps.net"))
        assertEquals("https://relay.jjapps.net/s/abc", online("https://relay.jjapps.net"))
        assertEquals("https://relay.jjapps.net/s/abc", online("https://relay.jjapps.net/"))
        assertEquals("https://relay.jjapps.net/s/abc", online("  relay.jjapps.net  "))
        // Uji lokal tanpa TLS harus tetap bisa.
        assertEquals("http://10.0.0.5:9010/s/abc", online("http://10.0.0.5:9010"))
    }

    @Test
    fun `jalur yang belum diatur tidak bisa dipilih`() {
        val hanyaLan = SavedServer(id = "s", name = "n", lan_host = "192.168.1.10")
        assertNull("ONLINE belum diatur", hanyaLan.toRemote(ConnMode.ONLINE))
        assertFalse(hanyaLan.supports(ConnMode.ONLINE))
        assertTrue(hanyaLan.supports(ConnMode.LOCAL))

        // Store ID kosong = alamat relay belum lengkap, jangan dianggap siap.
        val relayTanpaStore = SavedServer(id = "s", name = "n", relay_url = "relay.jjapps.net")
        assertFalse(relayTanpaStore.supports(ConnMode.ONLINE))
    }

    @Test
    fun `token perangkat dipakai di kedua jalur`() {
        val token = "t".repeat(64)
        val server = SavedServer(
            id = "s", name = "n",
            lan_host = "192.168.1.10",
            relay_url = "relay.jjapps.net", store_id = "abc",
            device_token = token,
        )
        assertEquals(token, server.toRemote(ConnMode.LOCAL)!!.token)
        assertEquals(token, server.toRemote(ConnMode.ONLINE)!!.token)
    }
}
