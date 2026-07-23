package com.galaxyas.mobilepos.ui.kasir

import android.content.Context
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
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.galaxyas.mobilepos.data.Session
import com.galaxyas.mobilepos.data.SettingsRepository
import com.galaxyas.mobilepos.data.model.Transaction
import com.galaxyas.mobilepos.data.model.TransactionDetail
import com.galaxyas.mobilepos.data.network.ApiClient
import com.galaxyas.mobilepos.printer.BtPrinter
import com.galaxyas.mobilepos.printer.buildReceiptEscPos
import com.galaxyas.mobilepos.printer.parseReceiptConfig
import com.galaxyas.mobilepos.util.formatDateTime
import com.galaxyas.mobilepos.util.formatIDR
import com.galaxyas.mobilepos.util.formatQty
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

data class HistoryState(
    val items: List<Transaction> = emptyList(),
    val total: Long = 0,
    val loading: Boolean = false,
    val search: String = "",
    val detail: TransactionDetail? = null,
    val message: String? = null,
) {
    val exhausted get() = items.size >= total
}

class HistoryViewModel(
    private val api: ApiClient,
    private val session: Session,
    private val settings: SettingsRepository,
    private val appContext: Context,
) : ViewModel() {
    private val _state = MutableStateFlow(HistoryState())
    val state = _state.asStateFlow()

    fun setSearch(q: String) { _state.value = _state.value.copy(search = q) }

    fun load(reset: Boolean) {
        val s = _state.value
        if (s.loading || (!reset && s.exhausted)) return
        _state.value = s.copy(loading = true)
        viewModelScope.launch {
            try {
                val page = api.listTransactionsPage(
                    limit = 50, offset = if (reset) 0 else s.items.size.toLong(),
                    search = s.search.ifBlank { null },
                )
                _state.value = _state.value.copy(
                    items = if (reset) page.items else s.items + page.items,
                    total = page.total, loading = false,
                )
            } catch (e: Exception) {
                _state.value = _state.value.copy(loading = false, message = e.message)
            }
        }
    }

    fun openDetail(id: String) {
        viewModelScope.launch {
            try {
                _state.value = _state.value.copy(detail = api.getTransaction(id))
            } catch (e: Exception) {
                _state.value = _state.value.copy(message = e.message)
            }
        }
    }

    fun closeDetail() { _state.value = _state.value.copy(detail = null) }
    fun clearMessage() { _state.value = _state.value.copy(message = null) }

    fun canVoid() = session.can("penjualan")

    fun voidTransaction(id: String) {
        viewModelScope.launch {
            try {
                api.deleteTransaction(id)
                _state.value = _state.value.copy(detail = null, message = "Transaksi dibatalkan.")
                load(reset = true)
            } catch (e: Exception) {
                _state.value = _state.value.copy(message = e.message)
            }
        }
    }

    fun reprint(onDone: (String) -> Unit) {
        val tx = _state.value.detail ?: return
        val cfg = parseReceiptConfig(settings.settings.value)
        val mac = cfg.printer
        if (mac.isNullOrBlank()) { onDone("Printer belum dipilih (Menu → Pengaturan → Printer)."); return }
        viewModelScope.launch {
            val res = BtPrinter.print(appContext, mac, buildReceiptEscPos(tx, cfg))
            onDone(res.fold({ "Struk dikirim ke printer." }, { it.message ?: "Gagal mencetak." }))
        }
    }
}

@Composable
fun TransactionHistoryScreen(vm: HistoryViewModel) {
    val state by vm.state.collectAsState()
    LaunchedEffect(Unit) { vm.load(reset = true) }
    LaunchedEffect(state.search) { vm.load(reset = true) }

    Column(Modifier.fillMaxSize().padding(horizontal = 12.dp)) {
        OutlinedTextField(
            value = state.search, onValueChange = { vm.setSearch(it) },
            label = { Text("Cari no. invoice") }, singleLine = true,
            modifier = Modifier.fillMaxWidth().padding(vertical = 8.dp),
        )
        LazyColumn(Modifier.fillMaxSize(), verticalArrangement = Arrangement.spacedBy(6.dp)) {
            items(state.items, key = { it.id }) { tx ->
                Card(Modifier.fillMaxWidth()) {
                    Row(
                        Modifier.fillMaxWidth().padding(12.dp),
                    ) {
                        Column(Modifier.weight(1f)) {
                            Text(tx.invoice_no, fontWeight = FontWeight.SemiBold)
                            Text(formatDateTime(tx.created_at),
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant)
                        }
                        Column(horizontalAlignment = androidx.compose.ui.Alignment.End) {
                            Text(formatIDR(tx.total), fontWeight = FontWeight.Bold)
                            TextButton(onClick = { vm.openDetail(tx.id) }) { Text("Detail") }
                        }
                    }
                }
            }
            item {
                val label = when {
                    state.loading -> "Memuat…"
                    state.items.isEmpty() -> "Belum ada transaksi."
                    state.exhausted -> "${state.total} transaksi."
                    else -> ""
                }
                if (label.isNotEmpty()) {
                    Text(label, color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(vertical = 10.dp))
                    LaunchedEffect(state.items.size) { if (!state.exhausted) vm.load(reset = false) }
                }
            }
        }
    }

    state.detail?.let { d -> TransactionDetailSheet(vm, d) }
    state.message?.let { m ->
        androidx.compose.material3.AlertDialog(
            onDismissRequest = { vm.clearMessage() },
            confirmButton = { TextButton(onClick = { vm.clearMessage() }) { Text("OK") } },
            text = { Text(m) },
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun TransactionDetailSheet(vm: HistoryViewModel, d: TransactionDetail) {
    var printing by remember { mutableStateOf(false) }
    var confirmVoid by remember { mutableStateOf(false) }
    ModalBottomSheet(onDismissRequest = { vm.closeDetail() }) {
        Column(Modifier.fillMaxWidth().padding(16.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
            Text(d.invoice_no, style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold)
            Text(formatDateTime(d.created_at), style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant)
            HorizontalDivider(Modifier.padding(vertical = 4.dp))
            d.items.forEach { it2 ->
                Row {
                    Text("${formatQty(it2.qty)}× ${it2.name}", Modifier.weight(1f),
                        style = MaterialTheme.typography.bodyMedium)
                    Text(formatIDR(it2.price * it2.qty - it2.discount), style = MaterialTheme.typography.bodyMedium)
                }
            }
            HorizontalDivider(Modifier.padding(vertical = 4.dp))
            Row { Text("Total", Modifier.weight(1f), fontWeight = FontWeight.Bold); Text(formatIDR(d.total), fontWeight = FontWeight.Bold) }
            Row { Text("Bayar (${d.payment_method})", Modifier.weight(1f)); Text(formatIDR(d.paid)) }
            Row { Text("Kembali", Modifier.weight(1f)); Text(formatIDR(d.change)) }

            Spacer(Modifier.width(4.dp))
            Row {
                if (vm.canVoid()) {
                    OutlinedButton(onClick = { confirmVoid = true }) {
                        Text("Batalkan", color = MaterialTheme.colorScheme.error)
                    }
                    Spacer(Modifier.width(8.dp))
                }
                Button(
                    onClick = { printing = true; vm.reprint { printing = false; vm.clearMessage(); } },
                    enabled = !printing, modifier = Modifier.weight(1f),
                ) { Text(if (printing) "Mencetak…" else "🖨️ Cetak Ulang") }
            }
            Spacer(Modifier.width(4.dp))
        }
    }
    if (confirmVoid) {
        androidx.compose.material3.AlertDialog(
            onDismissRequest = { confirmVoid = false },
            title = { Text("Batalkan transaksi?") },
            text = { Text("${d.invoice_no} akan dihapus dan stok dikembalikan. Tindakan ini tidak bisa dibatalkan.") },
            confirmButton = {
                TextButton(onClick = { confirmVoid = false; vm.voidTransaction(d.id) }) {
                    Text("Ya, batalkan", color = MaterialTheme.colorScheme.error)
                }
            },
            dismissButton = { TextButton(onClick = { confirmVoid = false }) { Text("Tidak") } },
        )
    }
}
