package com.galaxyas.mobilepos.ui.persediaan

import android.content.Context
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.QrCodeScanner
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
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
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.galaxyas.mobilepos.data.Session
import com.galaxyas.mobilepos.data.SettingsRepository
import com.galaxyas.mobilepos.data.model.ProductWithStock
import com.galaxyas.mobilepos.data.model.StockMovementBatch
import com.galaxyas.mobilepos.data.model.StockMovementBatchDetail
import com.galaxyas.mobilepos.data.model.StockMovementBatchInput
import com.galaxyas.mobilepos.data.model.StockMovementBatchItemInput
import com.galaxyas.mobilepos.data.network.ApiClient
import com.galaxyas.mobilepos.printer.BtPrinter
import com.galaxyas.mobilepos.printer.buildStockDocEscPos
import com.galaxyas.mobilepos.printer.parseReceiptConfig
import com.galaxyas.mobilepos.scanner.ScannerScreen
import com.galaxyas.mobilepos.ui.common.ConfirmDialog
import com.galaxyas.mobilepos.ui.common.FormField
import com.galaxyas.mobilepos.ui.common.MessageDialog
import com.galaxyas.mobilepos.util.formatDateTime
import com.galaxyas.mobilepos.util.formatQty
import kotlinx.coroutines.launch

/** Daftar batch Item Masuk / Keluar + entri baru + detail & cetak dokumen. */
@Composable
fun StockBatchScreen(
    api: ApiClient,
    session: Session,
    settings: SettingsRepository,
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()

    var kind by remember { mutableStateOf("in") }
    var items by remember { mutableStateOf<List<StockMovementBatch>>(emptyList()) }
    var detail by remember { mutableStateOf<StockMovementBatchDetail?>(null) }
    var creating by remember { mutableStateOf(false) }
    var toDelete by remember { mutableStateOf<StockMovementBatch?>(null) }
    var message by remember { mutableStateOf<String?>(null) }

    suspend fun reload() {
        runCatching { api.listStockMovementBatches(kind = kind, limit = 50, offset = 0) }
            .onSuccess { items = it.items }
            .onFailure { message = it.message }
    }
    LaunchedEffect(kind) { reload() }

    if (creating) {
        StockBatchEntryScreen(
            api = api, session = session, kind = kind,
            onDone = { creating = false; scope.launch { reload() } },
            onCancel = { creating = false },
        )
        return
    }

    Box(Modifier.fillMaxSize()) {
        Column(Modifier.fillMaxSize().padding(horizontal = 12.dp)) {
            Row(
                Modifier.padding(vertical = 8.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                FilterChip(kind == "in", { kind = "in" }, { Text("Item Masuk") })
                FilterChip(kind == "out", { kind = "out" }, { Text("Item Keluar") })
            }
            LazyColumn(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                items(items, key = { it.id }) { b ->
                    Card(Modifier.fillMaxWidth()) {
                        Row(Modifier.fillMaxWidth().padding(start = 12.dp), verticalAlignment = Alignment.CenterVertically) {
                            TextButton(
                                onClick = {
                                    scope.launch {
                                        runCatching { api.getStockMovementBatch(b.id) }
                                            .onSuccess { detail = it }
                                            .onFailure { message = it.message }
                                    }
                                },
                                modifier = Modifier.weight(1f),
                            ) {
                                Column(Modifier.fillMaxWidth()) {
                                    Text(b.no, fontWeight = FontWeight.SemiBold)
                                    Text(
                                        "${b.item_count} item · qty ${formatQty(b.total_qty)} · ${formatDateTime(b.created_at)}",
                                        style = MaterialTheme.typography.bodySmall,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    )
                                }
                            }
                            IconButton(onClick = { toDelete = b }) {
                                Icon(Icons.Default.Delete, "Hapus", tint = MaterialTheme.colorScheme.error)
                            }
                        }
                    }
                }
                if (items.isEmpty()) {
                    item {
                        Text("Belum ada dokumen.", color = MaterialTheme.colorScheme.onSurfaceVariant,
                            modifier = Modifier.padding(16.dp))
                    }
                }
            }
        }
        FloatingActionButton(
            onClick = { creating = true },
            modifier = Modifier.align(Alignment.BottomEnd).padding(16.dp),
        ) { Icon(Icons.Default.Add, "Buat dokumen") }
    }

    detail?.let { d ->
        BatchDetailSheet(
            detail = d,
            onPrint = {
                val cfg = parseReceiptConfig(settings.settings.value)
                val mac = cfg.printer
                if (mac.isNullOrBlank()) {
                    message = "Printer belum dipilih (Menu → Printer & Kertas)."
                } else {
                    scope.launch {
                        val res = BtPrinter.print(context, mac, buildStockDocEscPos(d, cfg))
                        message = res.fold({ "Dokumen dikirim ke printer." }, { it.message ?: "Gagal mencetak." })
                    }
                }
            },
            onClose = { detail = null },
        )
    }
    toDelete?.let { b ->
        ConfirmDialog(
            title = "Hapus dokumen?",
            text = "${b.no} akan dihapus dan efek stoknya dibatalkan.",
            confirmLabel = "Hapus", destructive = true,
            onConfirm = {
                scope.launch {
                    runCatching { api.deleteStockMovementBatch(b.id) }
                        .onSuccess { reload() }
                        .onFailure { message = it.message }
                }
            },
            onDismiss = { toDelete = null },
        )
    }
    message?.let { MessageDialog(it) { message = null } }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun BatchDetailSheet(
    detail: StockMovementBatchDetail,
    onPrint: () -> Unit,
    onClose: () -> Unit,
) {
    ModalBottomSheet(onDismissRequest = onClose) {
        Column(Modifier.fillMaxWidth().padding(16.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
            Text(detail.no, style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold)
            Text(
                "${if (detail.kind == "in") "Item Masuk" else "Item Keluar"} · ${formatDateTime(detail.created_at)}",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            HorizontalDivider(Modifier.padding(vertical = 4.dp))
            detail.items.forEach { it2 ->
                Row {
                    Text(it2.product_name, Modifier.weight(1f), style = MaterialTheme.typography.bodyMedium)
                    Text(formatQty(it2.qty), fontWeight = FontWeight.SemiBold)
                }
            }
            HorizontalDivider(Modifier.padding(vertical = 4.dp))
            Row {
                Text("Total Qty", Modifier.weight(1f), fontWeight = FontWeight.Bold)
                Text(formatQty(detail.items.sumOf { it.qty }), fontWeight = FontWeight.Bold)
            }
            detail.note?.let { Text("Catatan: $it", style = MaterialTheme.typography.bodySmall) }
            Spacer(Modifier.width(4.dp))
            Row {
                OutlinedButton(onClick = onClose, Modifier.weight(1f)) { Text("Tutup") }
                Spacer(Modifier.width(8.dp))
                Button(onClick = onPrint, Modifier.weight(1f)) { Text("🖨️ Cetak Dokumen") }
            }
            Spacer(Modifier.width(4.dp))
        }
    }
}

/** Entri batch baru: scan-to-add banyak barang sekaligus lalu simpan. */
@Composable
private fun StockBatchEntryScreen(
    api: ApiClient,
    session: Session,
    kind: String,
    onDone: () -> Unit,
    onCancel: () -> Unit,
) {
    data class Row2(val product: ProductWithStock, val qty: Double, val note: String)

    val scope = rememberCoroutineScope()
    var rows by remember { mutableStateOf<List<Row2>>(emptyList()) }
    var catatan by remember { mutableStateOf("") }
    var scanning by remember { mutableStateOf(false) }
    var busy by remember { mutableStateOf(false) }
    var message by remember { mutableStateOf<String?>(null) }
    var picking by remember { mutableStateOf(false) }

    if (scanning) {
        ScannerScreen(
            onResult = { code ->
                scope.launch {
                    val p = runCatching { api.findByBarcode(code) }.getOrNull()
                    if (p == null) {
                        message = "Barcode \"$code\" tidak ditemukan."
                    } else {
                        val existing = rows.firstOrNull { it.product.id == p.id }
                        rows = if (existing != null) {
                            rows.map { if (it.product.id == p.id) it.copy(qty = it.qty + 1) else it }
                        } else {
                            rows + Row2(p, 1.0, "")
                        }
                    }
                }
            },
            onClose = { scanning = false },
        )
        return
    }

    if (picking) {
        com.galaxyas.mobilepos.ui.kasir.ProductSearchSheet(
            api = api,
            onPick = { p ->
                val existing = rows.firstOrNull { it.product.id == p.id }
                rows = if (existing != null) rows.map { if (it.product.id == p.id) it.copy(qty = it.qty + 1) else it }
                else rows + Row2(p, 1.0, "")
                picking = false
            },
            onClose = { picking = false },
        )
    }

    Column(Modifier.fillMaxSize().padding(12.dp)) {
        Text(
            if (kind == "in") "Item Masuk baru" else "Item Keluar baru",
            style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold,
        )
        Row(Modifier.padding(vertical = 8.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Button(onClick = { scanning = true }) {
                Icon(Icons.Default.QrCodeScanner, contentDescription = null)
                Spacer(Modifier.width(6.dp))
                Text("Scan")
            }
            OutlinedButton(onClick = { picking = true }) { Text("Cari Barang") }
        }

        LazyColumn(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(6.dp)) {
            items(rows, key = { it.product.id }) { r ->
                Card(Modifier.fillMaxWidth()) {
                    Row(Modifier.fillMaxWidth().padding(horizontal = 10.dp, vertical = 6.dp),
                        verticalAlignment = Alignment.CenterVertically) {
                        Text(r.product.name, Modifier.weight(1f), fontWeight = FontWeight.SemiBold)
                        IconButton(onClick = {
                            rows = rows.map { if (it.product.id == r.product.id) it.copy(qty = (it.qty - 1).coerceAtLeast(1.0)) else it }
                        }) { Text("−", style = MaterialTheme.typography.titleLarge) }
                        Text(formatQty(r.qty), fontWeight = FontWeight.Bold)
                        IconButton(onClick = {
                            rows = rows.map { if (it.product.id == r.product.id) it.copy(qty = it.qty + 1) else it }
                        }) { Icon(Icons.Default.Add, "Tambah") }
                        IconButton(onClick = { rows = rows.filter { it.product.id != r.product.id } }) {
                            Icon(Icons.Default.Delete, "Hapus", tint = MaterialTheme.colorScheme.error)
                        }
                    }
                }
            }
            if (rows.isEmpty()) {
                item {
                    Text("Belum ada item — scan atau cari barang.",
                        color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.padding(16.dp))
                }
            }
        }

        FormField("Catatan dokumen", catatan, { catatan = it })
        Spacer(Modifier.width(8.dp))
        Row {
            OutlinedButton(onClick = onCancel) { Text("Batal") }
            Spacer(Modifier.width(8.dp))
            Button(
                onClick = {
                    if (rows.isEmpty()) { message = "Tidak ada item untuk disimpan."; return@Button }
                    busy = true
                    scope.launch {
                        try {
                            api.createStockMovementBatch(
                                StockMovementBatchInput(
                                    kind = kind,
                                    note = catatan.ifBlank { null },
                                    user_id = session.user.value?.username,
                                    items = rows.map {
                                        StockMovementBatchItemInput(it.product.id, it.qty, it.note.ifBlank { null })
                                    },
                                ),
                            )
                            busy = false
                            onDone()
                        } catch (e: Exception) {
                            busy = false
                            message = e.message
                        }
                    }
                },
                enabled = !busy, modifier = Modifier.weight(1f),
            ) { Text(if (busy) "Menyimpan…" else "Simpan (${rows.size} item)") }
        }
    }

    message?.let { MessageDialog(it) { message = null } }
}
