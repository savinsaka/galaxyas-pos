package com.galaxyas.mobilepos.ui.kasir

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.QrCodeScanner
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import com.galaxyas.mobilepos.data.model.Customer
import com.galaxyas.mobilepos.data.model.ProductWithStock
import com.galaxyas.mobilepos.data.network.ApiClient
import com.galaxyas.mobilepos.scanner.ScannerScreen
import com.galaxyas.mobilepos.util.formatIDR
import com.galaxyas.mobilepos.util.formatQty
import kotlinx.coroutines.launch

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun KasirScreen(vm: KasirViewModel, api: ApiClient) {
    val state by vm.state.collectAsState()
    val pending by vm.pending.collectAsState()

    var scanning by remember { mutableStateOf(false) }
    var showPayment by remember { mutableStateOf(false) }
    var showProductSearch by remember { mutableStateOf(false) }
    var showPending by remember { mutableStateOf(false) }
    var editLine by remember { mutableStateOf<CartLine?>(null) }

    LaunchedEffect(Unit) { vm.load() }

    if (scanning) {
        ScannerScreen(
            onResult = { vm.scanBarcode(it) },
            onClose = { scanning = false },
        )
        return
    }

    if (!state.shiftChecked) {
        Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) { Text("Memuat…") }
        return
    }

    if (state.activeShift == null) {
        ShiftGate(prefill = state.openingCashPrefill, busy = state.busy) { vm.openShift(it) }
        FlagsHost(state, vm)
        return
    }

    Box(Modifier.fillMaxSize()) {
        Column(Modifier.fillMaxSize()) {
            // Header: total besar + kembalian
            Surface(color = MaterialTheme.colorScheme.surfaceVariant) {
                Row(
                    Modifier.fillMaxWidth().padding(12.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Column(Modifier.weight(1f)) {
                        Text("Total", style = MaterialTheme.typography.labelMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant)
                        Text(
                            formatIDR(state.total),
                            style = MaterialTheme.typography.headlineMedium,
                            fontWeight = FontWeight.ExtraBold,
                            color = MaterialTheme.colorScheme.secondary,
                        )
                    }
                    if (pending.isNotEmpty()) {
                        TextButton(onClick = { showPending = true }) { Text("Pending (${pending.size})") }
                    }
                }
            }

            // Search + scan
            Row(
                Modifier.fillMaxWidth().padding(horizontal = 12.dp, vertical = 6.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                OutlinedButton(onClick = { showProductSearch = true }, modifier = Modifier.weight(1f)) {
                    Text("Cari nama barang…")
                }
                Spacer(Modifier.width(8.dp))
                FloatingActionButton(
                    onClick = { scanning = true },
                    modifier = Modifier.height(48.dp).width(48.dp),
                ) { Icon(Icons.Default.QrCodeScanner, contentDescription = "Scan") }
            }

            // Cart list
            if (state.cart.isEmpty()) {
                Box(Modifier.weight(1f).fillMaxWidth(), contentAlignment = Alignment.Center) {
                    Text(
                        "Belum ada item — scan barcode atau cari nama barang.",
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(24.dp),
                    )
                }
            } else {
                LazyColumn(
                    Modifier.weight(1f).fillMaxWidth().padding(horizontal = 12.dp),
                    verticalArrangement = Arrangement.spacedBy(6.dp),
                ) {
                    items(state.cart, key = { it.product_id }) { line ->
                        CartRow(
                            line = line,
                            onQty = { vm.setQty(line.product_id, it) },
                            onTap = { editLine = line },
                            onRemove = { vm.removeLine(line.product_id) },
                        )
                    }
                    item { Spacer(Modifier.height(8.dp)) }
                }
            }

            // Bottom bar
            Surface(shadowElevation = 8.dp) {
                Column(Modifier.fillMaxWidth().padding(12.dp)) {
                    Row {
                        Text("${formatQty(state.totalQty)} item · Subtotal ${formatIDR(state.subtotal)}",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            modifier = Modifier.weight(1f))
                        if (state.totalDiscount > 0) {
                            Text("Diskon −${formatIDR(state.totalDiscount)}",
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant)
                        }
                    }
                    Spacer(Modifier.height(6.dp))
                    Row {
                        if (state.cart.isNotEmpty()) {
                            OutlinedButton(onClick = { vm.holdCart() }) { Text("Tahan") }
                            Spacer(Modifier.width(8.dp))
                        }
                        Button(
                            onClick = { showPayment = true },
                            enabled = state.cart.isNotEmpty(),
                            modifier = Modifier.weight(1f),
                        ) { Text("Bayar ${formatIDR(state.total)}") }
                    }
                }
            }
        }
    }

    // Sheets & dialogs
    if (showProductSearch) {
        ProductSearchSheet(api = api, onPick = { vm.addProduct(it); showProductSearch = false }) {
            showProductSearch = false
        }
    }
    editLine?.let { line ->
        LineEditSheet(
            line = line,
            onQty = { vm.setQty(line.product_id, it) },
            onDiscount = { vm.setManualDiscount(line.product_id, it) },
            onRemove = { vm.removeLine(line.product_id); editLine = null },
            onClose = { editLine = null },
        )
    }
    if (showPayment) {
        PaymentSheet(
            state = state,
            customersFor = { q -> state.customers.filter { (it.phone ?: "").contains(q, ignoreCase = true) } },
            onSelectMethod = vm::selectPayment,
            onPaid = vm::setPaid, onPaidCash = vm::setPaidCash, onPaidQris = vm::setPaidQris,
            onCustomer = vm::setCustomer,
            onCheckout = { vm.checkout(); showPayment = false },
            onClose = { showPayment = false },
        )
    }
    if (showPending) {
        PendingSheet(
            pending = pending,
            onResume = { vm.resumePending(it); showPending = false },
            onDelete = { vm.deletePending(it) },
            onClose = { showPending = false },
        )
    }
    state.lastReceipt?.let { tx ->
        ReceiptSheet(
            tx = tx,
            onPrint = vm::printLastReceipt,
            onClose = { vm.dismissReceipt() },
        )
    }

    FlagsHost(state, vm)
}

/** Menampilkan toast/warning/stock error dari state via snackbar sederhana. */
@Composable
private fun FlagsHost(state: KasirState, vm: KasirViewModel) {
    // Ditangani di shell lewat toast global; di sini kita pakai dialog untuk
    // warning & stok karena butuh perhatian (checkout gagal, stok habis).
    val msg = state.warning ?: state.stockError?.let {
        if (it.available <= 0) "\"${it.name}\" stoknya kosong."
        else "Stok \"${it.name}\" tinggal ${formatQty(it.available)}, tidak cukup."
    }
    if (msg != null) {
        androidx.compose.material3.AlertDialog(
            onDismissRequest = { vm.clearFlags() },
            confirmButton = { TextButton(onClick = { vm.clearFlags() }) { Text("Tutup") } },
            title = { Text("Perhatian") },
            text = { Text(msg) },
        )
    }
}

@Composable
private fun CartRow(line: CartLine, onQty: (Double) -> Unit, onTap: () -> Unit, onRemove: () -> Unit) {
    Card(Modifier.fillMaxWidth()) {
        Row(
            Modifier.fillMaxWidth().padding(horizontal = 10.dp, vertical = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Column(
                Modifier.weight(1f).padding(end = 8.dp)
                    .clickable { onTap() },
            ) {
                Text(
                    line.name,
                    style = MaterialTheme.typography.bodyLarge,
                    fontWeight = FontWeight.SemiBold,
                )
                val sub = buildString {
                    append(formatIDR(line.price))
                    if (line.discount > 0) append("  •  −${formatIDR(line.discount)}")
                }
                Text(sub, style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            // Qty stepper
            IconButton(onClick = { onQty(line.qty - 1) }) { Text("−", style = MaterialTheme.typography.titleLarge) }
            Text(formatQty(line.qty), fontWeight = FontWeight.Bold)
            IconButton(onClick = { onQty(line.qty + 1) }) { Icon(Icons.Default.Add, contentDescription = "Tambah") }
            Text(
                formatIDR(line.lineTotal),
                fontWeight = FontWeight.Bold,
                modifier = Modifier.width(96.dp),
                textAlign = androidx.compose.ui.text.style.TextAlign.End,
            )
            IconButton(onClick = onRemove) {
                Icon(Icons.Default.Delete, contentDescription = "Hapus", tint = MaterialTheme.colorScheme.error)
            }
        }
    }
}

@Composable
private fun ShiftGate(prefill: Double, busy: Boolean, onOpen: (Double) -> Unit) {
    var cash by remember(prefill) { mutableStateOf(if (prefill > 0) prefill.toLong().toString() else "") }
    Column(
        Modifier.fillMaxSize().padding(24.dp),
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Text("🟢 Buka Shift Dulu", style = MaterialTheme.typography.headlineSmall)
        Text(
            "Masukkan modal awal (uang tunai di laci) sebelum mulai melayani transaksi.",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(vertical = 12.dp),
        )
        OutlinedTextField(
            value = cash, onValueChange = { cash = it.filter { c -> c.isDigit() } },
            label = { Text("Modal Awal (Rp)") }, singleLine = true, enabled = !busy,
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
            modifier = Modifier.fillMaxWidth(),
        )
        Spacer(Modifier.height(16.dp))
        Button(
            onClick = { onOpen(cash.toDoubleOrNull() ?: 0.0) },
            enabled = !busy, modifier = Modifier.fillMaxWidth(),
        ) { Text("Buka Shift & Mulai Jualan") }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun LineEditSheet(
    line: CartLine,
    onQty: (Double) -> Unit,
    onDiscount: (Double) -> Unit,
    onRemove: () -> Unit,
    onClose: () -> Unit,
) {
    var qty by remember { mutableStateOf(formatQty(line.qty)) }
    var disc by remember { mutableStateOf(line.discount.toLong().toString()) }
    ModalBottomSheet(onDismissRequest = onClose) {
        Column(Modifier.fillMaxWidth().padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
            Text(line.name, style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold)
            Text("Harga ${formatIDR(line.price)} · stok ${formatQty(line.stock_qty)}",
                style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            OutlinedTextField(
                value = qty, onValueChange = { qty = it },
                label = { Text("Jumlah") }, singleLine = true, modifier = Modifier.fillMaxWidth(),
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
            )
            OutlinedTextField(
                value = disc, onValueChange = { disc = it.filter { c -> c.isDigit() } },
                label = { Text("Diskon baris (Rp)") }, singleLine = true, modifier = Modifier.fillMaxWidth(),
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
            )
            Row {
                OutlinedButton(onClick = onRemove) {
                    Text("Hapus", color = MaterialTheme.colorScheme.error)
                }
                Spacer(Modifier.width(8.dp))
                Button(
                    onClick = {
                        qty.toDoubleOrNull()?.let(onQty)
                        disc.toDoubleOrNull()?.let(onDiscount)
                        onClose()
                    },
                    modifier = Modifier.weight(1f),
                ) { Text("Simpan") }
            }
            Spacer(Modifier.height(8.dp))
        }
    }
}
