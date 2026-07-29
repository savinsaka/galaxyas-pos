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
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.QrCodeScanner
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
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
import com.galaxyas.mobilepos.data.model.OpnameSpecialInput
import com.galaxyas.mobilepos.data.model.OpnameSpecialItemInput
import com.galaxyas.mobilepos.data.model.ProductWithStock
import com.galaxyas.mobilepos.data.network.ApiClient
import com.galaxyas.mobilepos.scanner.ScannerScreen
import com.galaxyas.mobilepos.ui.common.ConfirmDialog
import com.galaxyas.mobilepos.ui.common.FormField
import com.galaxyas.mobilepos.ui.common.MessageDialog
import com.galaxyas.mobilepos.ui.common.NumberField
import com.galaxyas.mobilepos.util.formatQty
import kotlinx.coroutines.launch

/**
 * Opname Spesial: pilih satu merek, scan barang yang benar-benar ada, isi stok
 * fisiknya. Saat disimpan, SEMUA barang merek itu yang tidak ada di daftar
 * dianggap habis — stoknya dinolkan server dalam satu transaksi
 * (`create_opname_special`), sama seperti OpnameSpesial.svelte di desktop.
 *
 * Barcode di luar merek yang dipilih ditolak (cuma notif), tidak masuk daftar.
 */
@Composable
fun OpnameSpecialScreen(api: ApiClient, session: Session) {
    data class OpRow(val product: ProductWithStock, val fisik: String)

    val scope = rememberCoroutineScope()
    var brands by remember { mutableStateOf<List<String>>(emptyList()) }
    var brand by remember { mutableStateOf<String?>(null) }
    var brandProducts by remember { mutableStateOf<List<ProductWithStock>>(emptyList()) }
    var loadingBrand by remember { mutableStateOf(false) }
    var rows by remember { mutableStateOf<List<OpRow>>(emptyList()) }
    var query by remember { mutableStateOf("") }
    var note by remember { mutableStateOf("") }
    var scanning by remember { mutableStateOf(false) }
    var busy by remember { mutableStateOf(false) }
    var confirming by remember { mutableStateOf(false) }
    var message by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(Unit) {
        brands = runCatching { api.listBrands().map { it.name } }.getOrDefault(emptyList())
    }
    // Filter merek dikerjakan server (list_products_page) — jangan tarik daftar
    // ber-limit lalu saring di klien, nanti barang di luar limit ikut hilang
    // padahal barang itulah yang akan dinolkan.
    LaunchedEffect(brand) {
        rows = emptyList()
        query = ""
        val b = brand
        if (b == null) { brandProducts = emptyList(); return@LaunchedEffect }
        loadingBrand = true
        brandProducts = runCatching {
            api.listProductsPage(brand = b, includeInactive = true, limit = 100_000L, offset = 0L).items
        }.getOrDefault(emptyList())
        loadingBrand = false
    }

    /** Stok sistem sebagai teks polos (tanpa pemisah ribuan) supaya bisa diedit & di-parse. */
    fun stockText(p: ProductWithStock): String =
        if (p.stock_qty % 1.0 == 0.0) p.stock_qty.toLong().toString() else p.stock_qty.toString()

    fun addProduct(p: ProductWithStock) {
        if (rows.any { it.product.id == p.id }) {
            message = "\"${p.name}\" sudah ada di daftar opname."
            return
        }
        rows = rows + OpRow(p, stockText(p))
        query = ""
    }

    if (scanning) {
        ScannerScreen(
            onResult = { code ->
                scope.launch {
                    // Cari di daftar merek dulu (ikut barang non-aktif, tanpa
                    // bolak-balik ke server); baru tanya server untuk bisa
                    // membedakan "beda merek" dari "tidak ada sama sekali".
                    val local = brandProducts.firstOrNull { it.barcode?.equals(code, true) == true }
                    if (local != null) {
                        addProduct(local)
                    } else {
                        val outside = runCatching { api.findByBarcode(code) }.getOrNull()
                        message = if (outside != null) {
                            "Barang beda merek: \"${outside.name}\" merek " +
                                "${outside.brand ?: "(tanpa merek)"}, bukan ${brand ?: "-"}."
                        } else {
                            "Barcode \"$code\" tidak ada di merek ${brand ?: "-"}."
                        }
                    }
                }
            },
            onClose = { scanning = false },
        )
        return
    }

    val countedIds = rows.map { it.product.id }.toSet()
    val toZero = brandProducts.filter { it.id !in countedIds && it.stock_qty != 0.0 }
    val searchHits =
        if (query.isBlank()) emptyList()
        else brandProducts.filter {
            it.name.contains(query, true) || (it.barcode ?: "").contains(query, true)
        }.take(20)

    fun simpan() {
        val b = brand ?: return
        val items = rows.map {
            val qty = it.fisik.toDoubleOrNull()
            if (qty == null || qty < 0) {
                message = "Stok fisik \"${it.product.name}\" belum diisi dengan benar."
                return
            }
            OpnameSpecialItemInput(it.product.id, qty)
        }
        busy = true
        scope.launch {
            try {
                val res = api.createOpnameSpecial(
                    OpnameSpecialInput(
                        brand = b,
                        note = note.ifBlank { null },
                        user_id = session.user.value?.username,
                        items = items,
                    ),
                )
                busy = false
                message = "Opname spesial merek \"${res.brand}\" selesai: " +
                    "${res.counted} barang dihitung, ${res.zeroed} barang dinolkan."
                rows = emptyList()
                note = ""
                brandProducts = runCatching {
                    api.listProductsPage(brand = b, includeInactive = true, limit = 100_000L, offset = 0L).items
                }.getOrDefault(brandProducts)
            } catch (e: Exception) {
                busy = false
                message = e.message
            }
        }
    }

    Column(Modifier.fillMaxSize().padding(horizontal = 12.dp)) {
        if (brands.isNotEmpty()) {
            Row(
                Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(top = 8.dp),
                horizontalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                brands.forEach { b ->
                    FilterChip(
                        selected = brand == b,
                        onClick = { brand = if (brand == b) null else b },
                        label = { Text(b) },
                    )
                }
            }
        }

        Card(
            Modifier.fillMaxWidth().padding(vertical = 8.dp),
            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.errorContainer),
        ) {
            Column(Modifier.padding(10.dp)) {
                Text(
                    "⚠️ Semua barang merek ini yang TIDAK ada di daftar bawah akan jadi stok 0 saat disimpan.",
                    style = MaterialTheme.typography.bodySmall,
                    fontWeight = FontWeight.SemiBold,
                    color = MaterialTheme.colorScheme.onErrorContainer,
                )
                Text(
                    when {
                        brand == null -> "Pilih merek dulu."
                        loadingBrand -> "Memuat daftar barang merek…"
                        else -> "${brandProducts.size} barang merek $brand · ${rows.size} dihitung · " +
                            "${toZero.size} akan dinolkan"
                    },
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onErrorContainer,
                )
            }
        }

        if (brand != null) {
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Button(onClick = { scanning = true }) {
                    Icon(Icons.Default.QrCodeScanner, contentDescription = null)
                    Spacer(Modifier.width(6.dp))
                    Text("Scan")
                }
                OutlinedTextField(
                    value = query,
                    onValueChange = { query = it },
                    label = { Text("Cari di merek ini") },
                    singleLine = true,
                    modifier = Modifier.weight(1f),
                )
            }

            if (searchHits.isNotEmpty()) {
                LazyColumn(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    items(searchHits, key = { it.id }) { p ->
                        Card(Modifier.fillMaxWidth()) {
                            TextButton(onClick = { addProduct(p) }, modifier = Modifier.fillMaxWidth()) {
                                Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                                    Column(Modifier.weight(1f)) {
                                        Text(p.name, fontWeight = FontWeight.SemiBold)
                                        Text(
                                            p.barcode ?: "tanpa barcode",
                                            style = MaterialTheme.typography.bodySmall,
                                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                                        )
                                    }
                                    Text(
                                        "stok ${formatQty(p.stock_qty)}",
                                        style = MaterialTheme.typography.bodySmall,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    )
                                }
                            }
                        }
                    }
                }
            } else {
                LazyColumn(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    items(rows, key = { it.product.id }) { r ->
                        val selisih = (r.fisik.toDoubleOrNull() ?: 0.0) - r.product.stock_qty
                        Card(Modifier.fillMaxWidth()) {
                            Column(Modifier.padding(10.dp)) {
                                Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                                    Column(Modifier.weight(1f)) {
                                        Text(r.product.name, fontWeight = FontWeight.SemiBold)
                                        Text(
                                            "stok sistem ${formatQty(r.product.stock_qty)}",
                                            style = MaterialTheme.typography.bodySmall,
                                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                                        )
                                    }
                                    IconButton(onClick = {
                                        rows = rows.filter { it.product.id != r.product.id }
                                    }) {
                                        Icon(Icons.Default.Delete, "Hapus", tint = MaterialTheme.colorScheme.error)
                                    }
                                }
                                NumberField(
                                    "Stok fisik",
                                    r.fisik,
                                    { v ->
                                        rows = rows.map {
                                            if (it.product.id == r.product.id) it.copy(fisik = v) else it
                                        }
                                    },
                                    decimal = true,
                                )
                                Text(
                                    "Selisih: ${if (selisih > 0) "+" else ""}${formatQty(selisih)}",
                                    style = MaterialTheme.typography.bodySmall,
                                    fontWeight = FontWeight.SemiBold,
                                    color = if (selisih == 0.0) MaterialTheme.colorScheme.onSurfaceVariant
                                    else MaterialTheme.colorScheme.error,
                                )
                            }
                        }
                    }
                    if (rows.isEmpty()) {
                        item {
                            Text(
                                "Belum ada barang dihitung — scan atau cari barang merek ini.",
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                modifier = Modifier.padding(16.dp),
                            )
                        }
                    }
                }
            }

            FormField("Keterangan", note, { note = it })
            Spacer(Modifier.width(8.dp))
            Button(
                onClick = { confirming = true },
                enabled = !busy && !loadingBrand,
                modifier = Modifier.fillMaxWidth().padding(bottom = 12.dp),
            ) {
                Text(if (busy) "Menyimpan…" else "Simpan Opname Spesial (${rows.size} dihitung)")
            }
        }
    }

    if (confirming) {
        ConfirmDialog(
            title = "Simpan Opname Spesial merek \"${brand ?: "-"}\"?",
            text = "${rows.size} barang diset sesuai hitungan fisik, dan ${toZero.size} barang " +
                "merek ini yang tidak dihitung stoknya DINOLKAN. Opname tidak bisa dibatalkan.",
            confirmLabel = "Ya, Simpan & Nolkan",
            destructive = true,
            onConfirm = { simpan() },
            onDismiss = { confirming = false },
        )
    }
    message?.let { MessageDialog(it) { message = null } }
}
