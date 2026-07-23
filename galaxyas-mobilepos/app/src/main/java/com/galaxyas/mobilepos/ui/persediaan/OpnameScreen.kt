package com.galaxyas.mobilepos.ui.persediaan

import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.QrCodeScanner
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.galaxyas.mobilepos.data.Session
import com.galaxyas.mobilepos.data.model.ProductWithStock
import com.galaxyas.mobilepos.data.model.StockMovementInput
import com.galaxyas.mobilepos.data.network.ApiClient
import com.galaxyas.mobilepos.scanner.ScannerScreen
import com.galaxyas.mobilepos.ui.common.FormField
import com.galaxyas.mobilepos.ui.common.MessageDialog
import com.galaxyas.mobilepos.ui.common.NumberField
import com.galaxyas.mobilepos.util.formatQty
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

/**
 * Stok Opname: pilih barang (scan/cari) → lihat stok sistem → isi stok fisik →
 * simpan sebagai pergerakan stok kind "opname" (qty = hasil hitung fisik,
 * bukan selisih) — sama seperti Opname.svelte di desktop.
 */
@Composable
fun OpnameScreen(api: ApiClient, session: Session) {
    val scope = rememberCoroutineScope()
    // null = mode cari/scan; terisi = mode "Opname per Merek" (susuri satu merek).
    var brandFilter by remember { mutableStateOf<String?>(null) }
    var brands by remember { mutableStateOf<List<String>>(emptyList()) }
    var query by remember { mutableStateOf("") }
    var results by remember { mutableStateOf<List<ProductWithStock>>(emptyList()) }
    var selected by remember { mutableStateOf<ProductWithStock?>(null) }
    var fisik by remember { mutableStateOf("") }
    var note by remember { mutableStateOf("") }
    var scanning by remember { mutableStateOf(false) }
    var busy by remember { mutableStateOf(false) }
    var message by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(Unit) {
        brands = runCatching { api.listBrands().map { it.name } }.getOrDefault(emptyList())
    }
    // Mode "per merek": langsung tampilkan semua barang merek tsb untuk disusuri.
    LaunchedEffect(brandFilter) {
        if (brandFilter != null) {
            results = runCatching { api.listProducts("", false, 500) }
                .getOrDefault(emptyList())
                .filter { it.brand == brandFilter }
        }
    }
    LaunchedEffect(query) {
        if (brandFilter != null) return@LaunchedEffect
        if (query.isBlank()) { results = emptyList(); return@LaunchedEffect }
        delay(300)
        results = runCatching { api.listProducts(query.trim(), false, 30) }.getOrDefault(emptyList())
    }

    if (scanning) {
        ScannerScreen(
            continuous = false,
            onResult = { code ->
                scanning = false
                scope.launch {
                    val p = runCatching { api.findByBarcode(code) }.getOrNull()
                    if (p == null) message = "Barcode \"$code\" tidak ditemukan."
                    else { selected = p; fisik = "" }
                }
            },
            onClose = { scanning = false },
        )
        return
    }

    fun simpan() {
        val p = selected ?: return
        val qty = fisik.toDoubleOrNull()
        if (qty == null) { message = "Isi jumlah fisik dulu."; return }
        busy = true
        scope.launch {
            try {
                api.createStockMovement(
                    StockMovementInput(
                        product_id = p.id,
                        kind = "opname",
                        qty = qty,
                        note = note.ifBlank { "Stok opname" },
                        user_id = session.user.value?.username,
                    ),
                )
                busy = false
                message = "Stok \"${p.name}\" disesuaikan jadi ${formatQty(qty)}."
                selected = null; fisik = ""; note = ""
                // Muat ulang daftar agar stok terbaru terlihat.
                if (brandFilter != null) {
                    results = runCatching { api.listProducts("", false, 500) }
                        .getOrDefault(emptyList()).filter { it.brand == brandFilter }
                } else if (query.isNotBlank()) {
                    results = runCatching { api.listProducts(query.trim(), false, 30) }.getOrDefault(emptyList())
                }
            } catch (e: Exception) {
                busy = false
                message = e.message
            }
        }
    }

    Column(Modifier.fillMaxSize().padding(horizontal = 12.dp)) {
        // Pemilih mode: cari/scan bebas, atau susuri satu merek (Opname per Merek).
        if (brands.isNotEmpty()) {
            Row(
                Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(top = 8.dp),
                horizontalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                FilterChip(
                    selected = brandFilter == null,
                    onClick = { brandFilter = null; results = emptyList() },
                    label = { Text("Cari / Scan") },
                )
                brands.forEach { b ->
                    FilterChip(
                        selected = brandFilter == b,
                        onClick = { brandFilter = b; selected = null },
                        label = { Text(b) },
                    )
                }
            }
        }
        if (brandFilter == null) {
            OutlinedTextField(
                value = query, onValueChange = { query = it },
                label = { Text("Cari barang") }, singleLine = true,
                modifier = Modifier.fillMaxWidth().padding(vertical = 8.dp),
                trailingIcon = {
                    IconButton(onClick = { scanning = true }) {
                        Icon(Icons.Default.QrCodeScanner, contentDescription = "Scan")
                    }
                },
            )
        } else {
            Text(
                "Opname merek: $brandFilter (${results.size} barang)",
                style = MaterialTheme.typography.titleSmall,
                modifier = Modifier.padding(vertical = 8.dp),
            )
        }

        selected?.let { p ->
            Card(Modifier.fillMaxWidth().padding(bottom = 8.dp)) {
                Column(Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(p.name, fontWeight = FontWeight.Bold)
                    Text(
                        "Stok sistem: ${formatQty(p.stock_qty)}",
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        style = MaterialTheme.typography.bodySmall,
                    )
                    NumberField("Stok Fisik (hasil hitung)", fisik, { fisik = it }, decimal = true)
                    fisik.toDoubleOrNull()?.let { f ->
                        val diff = f - p.stock_qty
                        Text(
                            "Selisih: ${if (diff > 0) "+" else ""}${formatQty(diff)}",
                            color = if (diff == 0.0) MaterialTheme.colorScheme.onSurfaceVariant
                            else MaterialTheme.colorScheme.error,
                            fontWeight = FontWeight.SemiBold,
                        )
                    }
                    FormField("Keterangan", note, { note = it })
                    Row {
                        OutlinedButton(onClick = { selected = null; fisik = "" }) { Text("Batal") }
                        Spacer(Modifier.width(8.dp))
                        Button(onClick = { simpan() }, enabled = !busy, modifier = Modifier.weight(1f)) {
                            Text(if (busy) "Menyimpan…" else "Simpan Opname")
                        }
                    }
                }
            }
        }

        LazyColumn(verticalArrangement = Arrangement.spacedBy(6.dp)) {
            items(results, key = { it.id }) { p ->
                Card(Modifier.fillMaxWidth()) {
                    TextButton(onClick = { selected = p; fisik = "" }, modifier = Modifier.fillMaxWidth()) {
                        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                            Text(p.name, Modifier.weight(1f), fontWeight = FontWeight.SemiBold)
                            Text("stok ${formatQty(p.stock_qty)}",
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant)
                        }
                    }
                }
            }
            if (results.isEmpty() && selected == null) {
                item {
                    Text(
                        if (brandFilter != null) "Tidak ada barang untuk merek ini."
                        else "Scan atau cari barang untuk mulai opname.",
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(16.dp),
                    )
                }
            }
        }
    }

    message?.let { MessageDialog(it) { message = null } }
}
