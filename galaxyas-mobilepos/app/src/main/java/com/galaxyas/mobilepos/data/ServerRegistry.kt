package com.galaxyas.mobilepos.data

import android.content.Context
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
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

/** Server Pusat tersimpan (padanan servers.json di client desktop). */
@Serializable
data class SavedServer(
    val id: String,
    val name: String,
    val host: String,
    val port: Int,
    val token: String,
) {
    fun toRemote() = RemoteConfig(host, port, token)
}

@Serializable
private data class Registry(
    val active_id: String = "",
    val servers: List<SavedServer> = emptyList(),
)

/**
 * Registry Server Pusat di Preferences DataStore. Belum ada server aktif =
 * frontend menampilkan onboarding pairing.
 */
class ServerRegistry(private val context: Context, scope: CoroutineScope) {
    private val key = stringPreferencesKey("registry")
    private val json = Json { ignoreUnknownKeys = true }

    private fun parse(raw: String?): Registry =
        raw?.let { runCatching { json.decodeFromString<Registry>(it) }.getOrNull() } ?: Registry()

    val servers: StateFlow<List<SavedServer>> = context.serverStore.data
        .map { parse(it[key]).servers }
        .stateIn(scope, SharingStarted.Eagerly, emptyList())

    val activeServer: StateFlow<SavedServer?> = context.serverStore.data
        .map { parse(it[key]).let { reg -> reg.servers.firstOrNull { s -> s.id == reg.active_id } } }
        .stateIn(scope, SharingStarted.Eagerly, null)

    /** Snapshot sinkron untuk RpcClient (tanpa suspend di jalur panggilan). */
    fun activeRemote(): RemoteConfig? = activeServer.value?.toRemote()

    private suspend fun update(transform: (Registry) -> Registry) {
        context.serverStore.edit { prefs ->
            prefs[key] = json.encodeToString(Registry.serializer(), transform(parse(prefs[key])))
        }
    }

    /** Simpan server baru; server pertama otomatis jadi aktif. */
    suspend fun add(name: String, host: String, port: Int, token: String): SavedServer {
        val server = SavedServer(UUID.randomUUID().toString(), name, host, port, token)
        update { reg ->
            reg.copy(
                servers = reg.servers + server,
                active_id = reg.active_id.ifEmpty { server.id },
            )
        }
        return server
    }

    suspend fun select(id: String) {
        update { reg -> if (reg.servers.any { it.id == id }) reg.copy(active_id = id) else reg }
    }

    suspend fun remove(id: String) {
        update { reg ->
            val remaining = reg.servers.filter { it.id != id }
            reg.copy(
                servers = remaining,
                active_id = if (reg.active_id == id) remaining.firstOrNull()?.id ?: "" else reg.active_id,
            )
        }
    }

    /** Tunggu load pertama dari disk (dipanggil splash/boot gate). */
    suspend fun awaitLoaded() {
        context.serverStore.data.first()
    }
}
