package com.galaxyas.mobilepos.data

import android.content.Context
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.stateIn

private val Context.settingsStore by preferencesDataStore(name = "settings")

/**
 * Pengaturan per-device sebagai map String->String dengan NAMA KEY SAMA seperti
 * tabel settings desktop (store_name, receipt_paper, receipt_show_*, theme, ...)
 * supaya parseReceiptConfig port jalan tanpa perubahan. Client LAN desktop pun
 * menyimpan pengaturannya per-device, jadi perilaku ini konsisten.
 * Key khusus mobile: bt_printer_mac, bt_printer_name.
 */
class SettingsRepository(private val context: Context, scope: CoroutineScope) {

    val settings: StateFlow<Map<String, String>> = context.settingsStore.data
        .map { prefs -> prefs.asMap().entries.associate { it.key.name to it.value.toString() } }
        .stateIn(scope, SharingStarted.Eagerly, emptyMap())

    suspend fun set(key: String, value: String) {
        context.settingsStore.edit { it[stringPreferencesKey(key)] = value }
    }

    fun get(key: String): String? = settings.value[key]
}
