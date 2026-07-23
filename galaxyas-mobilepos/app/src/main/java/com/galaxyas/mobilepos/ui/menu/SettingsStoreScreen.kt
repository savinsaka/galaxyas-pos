package com.galaxyas.mobilepos.ui.menu

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.galaxyas.mobilepos.data.SettingsRepository
import com.galaxyas.mobilepos.printer.RECEIPT_SHOW_KEYS
import com.galaxyas.mobilepos.ui.common.FormField
import com.galaxyas.mobilepos.ui.common.MessageDialog
import com.galaxyas.mobilepos.ui.common.SectionCard
import kotlinx.coroutines.launch

/**
 * Pengaturan Toko & Struk (per-device, sama seperti client LAN desktop).
 * Key setting identik dengan desktop sehingga parseReceiptConfig langsung pakai.
 */
@Composable
fun SettingsStoreScreen(settings: SettingsRepository) {
    val all by settings.settings.collectAsState()
    val scope = rememberCoroutineScope()
    var saved by remember { mutableStateOf<String?>(null) }

    // Nilai awal diambil sekali; simpan eksplisit lewat tombol.
    var storeName by remember(all["store_name"]) { mutableStateOf(all["store_name"] ?: "") }
    var address by remember(all["store_address"]) { mutableStateOf(all["store_address"] ?: "") }
    var phone by remember(all["store_phone"]) { mutableStateOf(all["store_phone"] ?: "") }
    var taxId by remember(all["store_tax_id"]) { mutableStateOf(all["store_tax_id"] ?: "") }
    var instagram by remember(all["store_instagram"]) { mutableStateOf(all["store_instagram"] ?: "") }
    var tiktok by remember(all["store_tiktok"]) { mutableStateOf(all["store_tiktok"] ?: "") }
    var whatsapp by remember(all["store_whatsapp"]) { mutableStateOf(all["store_whatsapp"] ?: "") }
    var header by remember(all["receipt_header"]) { mutableStateOf(all["receipt_header"] ?: "") }
    var footer by remember(all["receipt_footer"]) { mutableStateOf(all["receipt_footer"] ?: "") }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text(
            "Pengaturan ini tersimpan di HP ini saja (sama seperti kasir tambahan di PC lain) — " +
                "dipakai untuk tampilan struk.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )

        SectionCard("Info Toko") {
            FormField("Nama Toko", storeName, { storeName = it })
            FormField("Alamat", address, { address = it })
            FormField("Telepon", phone, { phone = it })
            FormField("NPWP", taxId, { taxId = it })
        }

        SectionCard("Sosial Media") {
            FormField("Instagram", instagram, { instagram = it })
            FormField("TikTok", tiktok, { tiktok = it })
            FormField("WhatsApp", whatsapp, { whatsapp = it })
        }

        SectionCard("Teks Struk") {
            FormField("Header tambahan", header, { header = it })
            FormField("Footer", footer, { footer = it })
        }

        Button(
            onClick = {
                scope.launch {
                    settings.set("store_name", storeName)
                    settings.set("store_address", address)
                    settings.set("store_phone", phone)
                    settings.set("store_tax_id", taxId)
                    settings.set("store_instagram", instagram)
                    settings.set("store_tiktok", tiktok)
                    settings.set("store_whatsapp", whatsapp)
                    settings.set("receipt_header", header)
                    settings.set("receipt_footer", footer)
                    saved = "Pengaturan tersimpan."
                }
            },
            modifier = Modifier.fillMaxWidth(),
        ) { Text("Simpan") }

        SectionCard("Blok yang Tampil di Struk") {
            RECEIPT_SHOW_KEYS.forEach { (key, label, _) ->
                val on = (all[key] ?: "1") != "0"
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(label, Modifier.weight(1f))
                    Switch(
                        checked = on,
                        onCheckedChange = { checked ->
                            scope.launch { settings.set(key, if (checked) "1" else "0") }
                        },
                    )
                }
            }
        }
        Spacer(Modifier.width(8.dp))
    }

    saved?.let { MessageDialog(it) { saved = null } }
}
