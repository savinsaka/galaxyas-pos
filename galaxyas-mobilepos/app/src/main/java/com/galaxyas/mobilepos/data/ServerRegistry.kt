package com.galaxyas.mobilepos.data

import android.content.Context
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import com.galaxyas.mobilepos.data.network.ConnMode
import com.galaxyas.mobilepos.data.network.RemoteConfig
import java.util.UUID
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.stateIn
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json

private val Context.serverStore by preferencesDataStore(name = "servers")

/**
 * Server Pusat tersimpan. Satu entri bisa menyimpan DUA alamat sekaligus:
 * alamat LAN (cepat, jalan walau internet toko mati) dan alamat relay (bisa
 * dipakai dari mana saja). Yang dipakai ditentukan pilihan LOCAL/ONLINE di
 * layar awal, bukan ditebak otomatis — permintaan pemilik toko.
 */
@Serializable
data class SavedServer(
    val id: String,
    val name: String,
    /** Alamat LAN. */
    val lan_host: String = "",
    val lan_port: Int = 8899,
    /** Alamat relay, mis. `https://relay.jjapps.net`. */
    val relay_url: String = "",
    val store_id: String = "",
    /** Token per-perangkat dari `/pair`; berlaku di kedua jalur. */
    val device_token: String = "",

    // --- Bentuk lama (sebelum ada Akses Online). Hanya dibaca sekali oleh
    // migrasi di bawah, tidak pernah ditulis lagi. ---
    val host: String? = null,
    val port: Int? = null,
    val token: String? = null,
) {
    val hasLan: Boolean get() = lan_host.isNotBlank()
    val hasOnline: Boolean get() = relay_url.isNotBlank() && store_id.isNotBlank()

    fun supports(mode: ConnMode): Boolean = when (mode) {
        ConnMode.LOCAL -> hasLan
        ConnMode.ONLINE -> hasOnline
    }

    /** Alamat yang ditampilkan di layar pilih koneksi. */
    fun addressOf(mode: ConnMode): String = when (mode) {
        ConnMode.LOCAL -> if (hasLan) "$lan_host:$lan_port" else "belum diatur"
        ConnMode.ONLINE -> if (hasOnline) relay_url.trim().trimEnd('/') else "belum diatur"
    }

    fun toRemote(mode: ConnMode): RemoteConfig? {
        if (!supports(mode)) return null
        val base = when (mode) {
            ConnMode.LOCAL -> "http://$lan_host:$lan_port"
            ConnMode.ONLINE -> "${normalizedRelayUrl()}/s/$store_id"
        }
        return RemoteConfig(baseUrl = base, token = device_token, mode = mode)
    }

    /** Terima ketikan `relay.jjapps.net` maupun `https://relay.jjapps.net/`. */
    private fun normalizedRelayUrl(): String {
        val raw = relay_url.trim().trimEnd('/')
        return if (raw.startsWith("http://") || raw.startsWith("https://")) raw else "https://$raw"
    }

    /**
     * Entri versi lama menyimpan `host/port/token` dan hanya punya jalur LAN.
     * Dipindahkan ke field baru supaya HP yang sudah terpasang tidak perlu
     * pairing ulang. `token` lama adalah kode pairing 6 karakter — masih sah di
     * jalur LAN (lihat lan.rs::authenticate), jadi aman dipakai apa adanya.
     */
    fun migrated(): SavedServer =
        if (host == null && port == null && token == null) {
            this
        } else {
            copy(
                lan_host = lan_host.ifBlank { host.orEmpty() },
                lan_port = if (lan_port == 8899 && port != null) port else lan_port,
                device_token = device_token.ifBlank { token.orEmpty() },
                host = null,
                port = null,
                token = null,
            )
        }
}

@Serializable
private data class Registry(
    val active_id: String = "",
    val active_mode: String = ConnMode.LOCAL.name,
    val servers: List<SavedServer> = emptyList(),
)

/**
 * Registry Server Pusat di Preferences DataStore. Belum ada server aktif =
 * frontend menampilkan onboarding pairing.
 */
class ServerRegistry(private val context: Context, scope: CoroutineScope) {
    private val key = stringPreferencesKey("registry")
    private val json = Json { ignoreUnknownKeys = true; encodeDefaults = true }

    private fun parse(raw: String?): Registry {
        val reg = raw?.let { runCatching { json.decodeFromString<Registry>(it) }.getOrNull() }
            ?: Registry()
        return reg.copy(servers = reg.servers.map { it.migrated() })
    }

    val servers: StateFlow<List<SavedServer>> = context.serverStore.data
        .map { parse(it[key]).servers }
        .stateIn(scope, SharingStarted.Eagerly, emptyList())

    val activeServer: StateFlow<SavedServer?> = context.serverStore.data
        .map { parse(it[key]).let { reg -> reg.servers.firstOrNull { s -> s.id == reg.active_id } } }
        .stateIn(scope, SharingStarted.Eagerly, null)

    val activeMode: StateFlow<ConnMode> = context.serverStore.data
        .map { ConnMode.parse(parse(it[key]).active_mode) }
        .stateIn(scope, SharingStarted.Eagerly, ConnMode.LOCAL)

    /** Snapshot sinkron untuk RpcClient (tanpa suspend di jalur panggilan). */
    fun activeRemote(): RemoteConfig? = activeServer.value?.toRemote(activeMode.value)

    private suspend fun update(transform: (Registry) -> Registry) {
        context.serverStore.edit { prefs ->
            prefs[key] = json.encodeToString(Registry.serializer(), transform(parse(prefs[key])))
        }
    }

    /** Simpan server baru; server pertama otomatis jadi aktif. */
    suspend fun add(server: SavedServer, mode: ConnMode): SavedServer {
        update { reg ->
            reg.copy(
                servers = reg.servers + server,
                active_id = server.id,
                active_mode = mode.name,
            )
        }
        return server
    }

    /** Buat entri baru dengan id acak (dipakai layar pairing). */
    fun newServer(
        name: String,
        lanHost: String,
        lanPort: Int,
        relayUrl: String,
        storeId: String,
        deviceToken: String,
    ) = SavedServer(
        id = UUID.randomUUID().toString(),
        name = name,
        lan_host = lanHost,
        lan_port = lanPort,
        relay_url = relayUrl,
        store_id = storeId,
        device_token = deviceToken,
    )

    suspend fun select(id: String, mode: ConnMode) {
        update { reg ->
            if (reg.servers.any { it.id == id }) {
                reg.copy(active_id = id, active_mode = mode.name)
            } else {
                reg
            }
        }
    }

    /** Ganti jalur tanpa mengganti server (tombol LOCAL/ONLINE). */
    suspend fun setMode(mode: ConnMode) {
        update { reg -> reg.copy(active_mode = mode.name) }
    }

    suspend fun remove(id: String) {
        update { reg ->
            val remaining = reg.servers.filter { it.id != id }
            reg.copy(
                servers = remaining,
                active_id = if (reg.active_id == id) "" else reg.active_id,
            )
        }
    }

    /** Tunggu load pertama dari disk (dipanggil splash/boot gate). */
    suspend fun awaitLoaded() {
        context.serverStore.data.first()
    }
}
