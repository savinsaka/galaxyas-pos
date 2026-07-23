package com.galaxyas.mobilepos.ui.persediaan

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
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
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
import com.galaxyas.mobilepos.data.model.Expense
import com.galaxyas.mobilepos.data.model.ExpenseInput
import com.galaxyas.mobilepos.data.network.ApiClient
import com.galaxyas.mobilepos.ui.common.ConfirmDialog
import com.galaxyas.mobilepos.ui.common.FormField
import com.galaxyas.mobilepos.ui.common.MessageDialog
import com.galaxyas.mobilepos.ui.common.NumberField
import com.galaxyas.mobilepos.util.formatIDR
import com.galaxyas.mobilepos.util.todayIso
import kotlinx.coroutines.launch

/** Pengeluaran (kas keluar) bulan berjalan: list + tambah + hapus. */
@Composable
fun ExpensesScreen(api: ApiClient, session: Session) {
    val scope = rememberCoroutineScope()
    var items by remember { mutableStateOf<List<Expense>>(emptyList()) }
    var adding by remember { mutableStateOf(false) }
    var toDelete by remember { mutableStateOf<Expense?>(null) }
    var message by remember { mutableStateOf<String?>(null) }

    suspend fun reload() {
        runCatching { api.listExpenses() }
            .onSuccess { items = it }
            .onFailure { message = it.message }
    }
    LaunchedEffect(Unit) { reload() }

    val total = items.sumOf { it.amount }

    Box(Modifier.fillMaxSize()) {
        Column(Modifier.fillMaxSize().padding(horizontal = 12.dp)) {
            Card(Modifier.fillMaxWidth().padding(vertical = 8.dp)) {
                Row(Modifier.padding(12.dp)) {
                    Text("Total pengeluaran", Modifier.weight(1f))
                    Text(formatIDR(total), fontWeight = FontWeight.Bold)
                }
            }
            LazyColumn(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                items(items, key = { it.id }) { e ->
                    Card(Modifier.fillMaxWidth()) {
                        Row(Modifier.fillMaxWidth().padding(start = 12.dp), verticalAlignment = Alignment.CenterVertically) {
                            Column(Modifier.weight(1f).padding(vertical = 10.dp)) {
                                Text(e.category, fontWeight = FontWeight.SemiBold)
                                Text(
                                    listOfNotNull(e.date, e.note).joinToString(" · "),
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                            Text(formatIDR(e.amount), fontWeight = FontWeight.Bold)
                            IconButton(onClick = { toDelete = e }) {
                                Icon(Icons.Default.Delete, "Hapus", tint = MaterialTheme.colorScheme.error)
                            }
                        }
                    }
                }
                if (items.isEmpty()) {
                    item {
                        Text("Belum ada pengeluaran.", color = MaterialTheme.colorScheme.onSurfaceVariant,
                            modifier = Modifier.padding(16.dp))
                    }
                }
            }
        }
        FloatingActionButton(
            onClick = { adding = true },
            modifier = Modifier.align(Alignment.BottomEnd).padding(16.dp),
        ) { Icon(Icons.Default.Add, "Tambah pengeluaran") }
    }

    if (adding) {
        ExpenseSheet(
            onSave = { input ->
                scope.launch {
                    runCatching { api.saveExpense(input) }
                        .onSuccess { reload() }
                        .onFailure { message = it.message }
                }
                adding = false
            },
            onDismiss = { adding = false },
            onError = { message = it },
            userId = session.user.value?.username,
        )
    }
    toDelete?.let { e ->
        ConfirmDialog(
            title = "Hapus pengeluaran?", text = "${e.category} — ${formatIDR(e.amount)}",
            confirmLabel = "Hapus", destructive = true,
            onConfirm = {
                scope.launch {
                    runCatching { api.deleteExpense(e.id) }.onSuccess { reload() }
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
private fun ExpenseSheet(
    onSave: (ExpenseInput) -> Unit,
    onDismiss: () -> Unit,
    onError: (String) -> Unit,
    userId: String?,
) {
    var category by remember { mutableStateOf("") }
    var amount by remember { mutableStateOf("") }
    var note by remember { mutableStateOf("") }
    var date by remember { mutableStateOf(todayIso()) }

    ModalBottomSheet(onDismissRequest = onDismiss) {
        Column(Modifier.fillMaxWidth().padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
            Text("Tambah Pengeluaran", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold)
            FormField("Kategori", category, { category = it })
            NumberField("Jumlah (Rp)", amount, { amount = it })
            FormField("Tanggal (YYYY-MM-DD)", date, { date = it })
            FormField("Catatan", note, { note = it })
            Button(
                onClick = {
                    val amt = amount.toDoubleOrNull()
                    when {
                        category.isBlank() -> onError("Kategori wajib diisi.")
                        amt == null || amt <= 0 -> onError("Jumlah harus lebih dari 0.")
                        else -> onSave(
                            ExpenseInput(
                                date = date, category = category.trim(), amount = amt,
                                note = note.trim().ifBlank { null }, user_id = userId,
                            ),
                        )
                    }
                },
                modifier = Modifier.fillMaxWidth(),
            ) { Text("Simpan") }
            Spacer(Modifier.width(4.dp))
        }
    }
}
