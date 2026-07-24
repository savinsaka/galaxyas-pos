package com.galaxyas.mobilepos.ui.kasir

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.AssistChip
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import com.galaxyas.mobilepos.data.PendingSale
import com.galaxyas.mobilepos.data.model.Customer
import com.galaxyas.mobilepos.data.model.ProductWithStock
import com.galaxyas.mobilepos.data.model.TransactionDetail
import com.galaxyas.mobilepos.data.network.ApiClient
import com.galaxyas.mobilepos.printer.ReceiptConfig
import com.galaxyas.mobilepos.printer.ReceiptShowFlags
import com.galaxyas.mobilepos.util.formatIDR
import com.galaxyas.mobilepos.util.formatQty
import kotlinx.coroutines.delay

private val PAYMENT_METHODS = listOf("Tunai", "QRIS", "Kombinasi", "Kartu")

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PaymentSheet(
    state: KasirState,
    customersFor: (String) -> List<Customer>,
    onSelectMethod: (String) -> Unit,
    onPaid: (Double) -> Unit,
    onPaidCash: (Double) -> Unit,
    onPaidQris: (Double) -> Unit,
    onCustomer: (Customer?) -> Unit,
    onCheckout: () -> Unit,
    onClose: () -> Unit,
) {
    ModalBottomSheet(onDismissRequest = onClose) {
        Column(Modifier.fillMaxWidth().padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
            Row {
                Text("Total", Modifier.weight(1f), style = MaterialTheme.typography.titleMedium)
                Text(formatIDR(state.total), style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold)
            }
            HorizontalDivider()

            // Metode
            Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                PAYMENT_METHODS.forEach { m ->
                    FilterChip(
                        selected = state.paymentMethod == m,
                        onClick = { onSelectMethod(m) },
                        label = { Text(m) },
                    )
                }
            }

            if (state.paymentMethod == "Kombinasi") {
                MoneyField("QRIS", state.paidQris, onPaidQris)
                MoneyField("Tunai", state.paidCash, onPaidCash)
            } else {
                MoneyField("Bayar", state.paid, onPaid)
                Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    OutlinedButton(onClick = { onPaid(state.total) }, Modifier.weight(1f)) { Text("Uang Pas") }
                    OutlinedButton(onClick = { onPaid(50_000.0) }, Modifier.weight(1f)) { Text("50rb") }
                    OutlinedButton(onClick = { onPaid(100_000.0) }, Modifier.weight(1f)) { Text("100rb") }
                }
            }

            Row {
                Text("Kembalian", Modifier.weight(1f), color = MaterialTheme.colorScheme.onSurfaceVariant)
                Text(formatIDR(state.change), fontWeight = FontWeight.Bold)
            }

            // Pelanggan (cari No. HP)
            CustomerPicker(
                selected = state.selectedCustomer,
                resultsFor = customersFor,
                onSelect = onCustomer,
            )

            Button(
                onClick = onCheckout,
                enabled = !state.busy && state.paid >= state.total,
                modifier = Modifier.fillMaxWidth(),
            ) { Text(if (state.busy) "Memproses…" else "Bayar & Simpan") }
            Spacer(Modifier.width(4.dp))
        }
    }
}

@Composable
private fun MoneyField(label: String, value: Double, onChange: (Double) -> Unit) {
    var text by remember(value) { mutableStateOf(if (value > 0) value.toLong().toString() else "") }
    OutlinedTextField(
        value = text,
        onValueChange = { s -> text = s.filter { it.isDigit() }; onChange(text.toDoubleOrNull() ?: 0.0) },
        label = { Text(label) }, singleLine = true, modifier = Modifier.fillMaxWidth(),
        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
    )
}

@Composable
private fun CustomerPicker(
    selected: Customer?,
    resultsFor: (String) -> List<Customer>,
    onSelect: (Customer?) -> Unit,
) {
    var query by remember { mutableStateOf("") }
    if (selected != null) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text("👤 ${selected.name}", Modifier.weight(1f))
            TextButton(onClick = { onSelect(null) }) { Text("Ganti") }
        }
    } else {
        OutlinedTextField(
            value = query, onValueChange = { query = it },
            label = { Text("Pelanggan (opsional, cari No. HP)") },
            singleLine = true, modifier = Modifier.fillMaxWidth(),
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Phone),
        )
        val results = if (query.isBlank()) emptyList() else resultsFor(query.trim())
        results.take(5).forEach { c ->
            TextButton(onClick = { onSelect(c); query = "" }, Modifier.fillMaxWidth()) {
                Text("${c.phone ?: ""} · ${c.name}", Modifier.fillMaxWidth())
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ProductSearchSheet(api: ApiClient, onPick: (ProductWithStock) -> Unit, onClose: () -> Unit) {
    var query by remember { mutableStateOf("") }
    var results by remember { mutableStateOf<List<ProductWithStock>>(emptyList()) }
    var loading by remember { mutableStateOf(false) }

    LaunchedEffect(query) {
        if (query.isBlank()) { results = emptyList(); return@LaunchedEffect }
        delay(300)
        loading = true
        results = runCatching { api.listProducts(query.trim(), false, 30) }.getOrDefault(emptyList())
        loading = false
    }

    ModalBottomSheet(onDismissRequest = onClose) {
        Column(Modifier.fillMaxWidth().padding(16.dp)) {
            OutlinedTextField(
                value = query, onValueChange = { query = it },
                label = { Text("Cari nama / barcode") }, singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(Modifier.width(8.dp))
            LazyColumn(Modifier.fillMaxWidth().heightIn(max = 380.dp)) {
                items(results, key = { it.id }) { p ->
                    TextButton(onClick = { onPick(p) }, Modifier.fillMaxWidth()) {
                        Row(Modifier.fillMaxWidth()) {
                            Column(Modifier.weight(1f)) {
                                Text(p.name, fontWeight = FontWeight.SemiBold)
                                // Barcode di bawah nama — memudahkan memastikan
                                // barang yang benar saat nama-nama mirip.
                                Text("${p.barcode ?: "tanpa barcode"} · stok ${formatQty(p.stock_qty)}",
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant)
                            }
                            Text(formatIDR(p.sell_price), fontWeight = FontWeight.Bold)
                        }
                    }
                    HorizontalDivider()
                }
                if (results.isEmpty()) {
                    item {
                        Text(
                            if (loading) "Mencari…" else if (query.isBlank()) "Ketik untuk mencari…" else "Tidak ditemukan.",
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            modifier = Modifier.padding(12.dp),
                        )
                    }
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PendingSheet(
    pending: List<PendingSale>,
    onResume: (String) -> Unit,
    onDelete: (String) -> Unit,
    onClose: () -> Unit,
) {
    ModalBottomSheet(onDismissRequest = onClose) {
        Column(Modifier.fillMaxWidth().padding(16.dp)) {
            Text("📋 Transaksi Pending", style = MaterialTheme.typography.titleMedium)
            Spacer(Modifier.width(8.dp))
            if (pending.isEmpty()) {
                Text("Tidak ada transaksi pending.", color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(12.dp))
            }
            pending.forEach { p ->
                Card(Modifier.fillMaxWidth().padding(vertical = 4.dp)) {
                    Row(Modifier.padding(12.dp), verticalAlignment = Alignment.CenterVertically) {
                        Column(Modifier.weight(1f)) {
                            Text(p.label, fontWeight = FontWeight.SemiBold)
                            val itemsQty = p.cart.sumOf { it.qty }
                            val total = p.cart.sumOf { it.price * it.qty - it.discount }
                            Text("${formatQty(itemsQty)} item · ${formatIDR(total)}",
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant)
                        }
                        TextButton(onClick = { onDelete(p.id) }) {
                            Text("Hapus", color = MaterialTheme.colorScheme.error)
                        }
                        Button(onClick = { onResume(p.id) }) { Text("Lanjut") }
                    }
                }
            }
            Spacer(Modifier.width(8.dp))
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ReceiptSheet(
    tx: TransactionDetail,
    onPrint: ((Boolean, String) -> Unit) -> Unit,
    onClose: () -> Unit,
) {
    var msg by remember { mutableStateOf<String?>(null) }
    var printing by remember { mutableStateOf(false) }
    ModalBottomSheet(onDismissRequest = onClose) {
        Column(Modifier.fillMaxWidth().padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Text("✅ Transaksi Tersimpan", style = MaterialTheme.typography.titleMedium)
            Text("${tx.invoice_no} · ${formatIDR(tx.total)}",
                color = MaterialTheme.colorScheme.onSurfaceVariant)
            // Preview ringkas isi struk
            Card(Modifier.fillMaxWidth()) {
                Column(Modifier.padding(12.dp)) {
                    tx.items.forEach { it2 ->
                        Row {
                            Text("${formatQty(it2.qty)}× ${it2.name}", Modifier.weight(1f),
                                style = MaterialTheme.typography.bodySmall)
                            Text(formatIDR(it2.price * it2.qty - it2.discount),
                                style = MaterialTheme.typography.bodySmall)
                        }
                    }
                    HorizontalDivider(Modifier.padding(vertical = 4.dp))
                    Row {
                        Text("Total", Modifier.weight(1f), fontWeight = FontWeight.Bold)
                        Text(formatIDR(tx.total), fontWeight = FontWeight.Bold)
                    }
                }
            }
            msg?.let { Text(it, color = MaterialTheme.colorScheme.onSurfaceVariant) }
            Row {
                OutlinedButton(onClick = onClose, Modifier.weight(1f)) { Text("Tutup") }
                Spacer(Modifier.width(8.dp))
                Button(
                    onClick = {
                        printing = true
                        onPrint { ok, m -> printing = false; msg = m; if (ok) onClose() }
                    },
                    enabled = !printing,
                    modifier = Modifier.weight(1f),
                ) { Text(if (printing) "Mencetak…" else "🖨️ Cetak Struk") }
            }
            Spacer(Modifier.width(4.dp))
        }
    }
}
