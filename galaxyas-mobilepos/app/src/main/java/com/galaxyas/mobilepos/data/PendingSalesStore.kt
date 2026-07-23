package com.galaxyas.mobilepos.data

import android.content.Context
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import com.galaxyas.mobilepos.ui.kasir.CartLine
import java.util.UUID
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.stateIn
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json

private val Context.pendingStore by preferencesDataStore(name = "pending_sales")

/** Transaksi ditahan (hold). DIPERSIST ke DataStore — HP bisa dibunuh OS, jadi
 *  tidak boleh cuma in-memory seperti desktop. */
@Serializable
data class PendingSale(
    val id: String = UUID.randomUUID().toString(),
    val label: String,
    val cart: List<CartLine>,
    val customerId: String? = null,
    val paymentMethod: String = "Tunai",
    val paid: Double = 0.0,
    val paidCash: Double = 0.0,
    val paidQris: Double = 0.0,
    val createdAt: Long = System.currentTimeMillis(),
)

class PendingSalesStore(private val context: Context, scope: CoroutineScope) {
    private val key = stringPreferencesKey("list")
    private val json = Json { ignoreUnknownKeys = true }

    private fun parse(raw: String?): List<PendingSale> =
        raw?.let { runCatching { json.decodeFromString<List<PendingSale>>(it) }.getOrNull() } ?: emptyList()

    val pending: StateFlow<List<PendingSale>> = context.pendingStore.data
        .map { parse(it[key]) }
        .stateIn(scope, SharingStarted.Eagerly, emptyList())

    private suspend fun update(transform: (List<PendingSale>) -> List<PendingSale>) {
        context.pendingStore.edit { prefs ->
            prefs[key] = json.encodeToString(transform(parse(prefs[key])))
        }
    }

    suspend fun add(sale: PendingSale) = update { it + sale }
    suspend fun remove(id: String) = update { list -> list.filter { it.id != id } }
}
