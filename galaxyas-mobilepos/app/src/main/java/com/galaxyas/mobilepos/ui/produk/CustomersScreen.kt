package com.galaxyas.mobilepos.ui.produk

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
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
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
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import com.galaxyas.mobilepos.data.model.Customer
import com.galaxyas.mobilepos.data.model.CustomerInput
import com.galaxyas.mobilepos.data.network.ApiClient
import com.galaxyas.mobilepos.ui.common.ConfirmDialog
import com.galaxyas.mobilepos.ui.common.FormField
import com.galaxyas.mobilepos.ui.common.MessageDialog
import kotlinx.coroutines.launch

/** Daftar Pelanggan: cari, tambah/edit (sheet), hapus. No. HP wajib (dipakai
 *  verifikasi di kasir — sama seperti aturan desktop). */
@Composable
fun CustomersScreen(api: ApiClient) {
    val scope = rememberCoroutineScope()
    var items by remember { mutableStateOf<List<Customer>>(emptyList()) }
    var search by remember { mutableStateOf("") }
    var editing by remember { mutableStateOf<Customer?>(null) }
    var adding by remember { mutableStateOf(false) }
    var toDelete by remember { mutableStateOf<Customer?>(null) }
    var message by remember { mutableStateOf<String?>(null) }

    suspend fun reload() {
        runCatching { api.listCustomers(search.trim()) }
            .onSuccess { items = it }
            .onFailure { message = it.message }
    }
    LaunchedEffect(search) { reload() }

    Box(Modifier.fillMaxSize()) {
        Column(Modifier.fillMaxSize().padding(horizontal = 12.dp)) {
            OutlinedTextField(
                value = search, onValueChange = { search = it },
                label = { Text("Cari nama / No. HP") }, singleLine = true,
                modifier = Modifier.fillMaxWidth().padding(vertical = 8.dp),
            )
            LazyColumn(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                items(items, key = { it.id }) { c ->
                    Card(Modifier.fillMaxWidth()) {
                        Row(
                            Modifier.fillMaxWidth().padding(start = 12.dp),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            TextButton(onClick = { editing = c }, modifier = Modifier.weight(1f)) {
                                Column(Modifier.fillMaxWidth()) {
                                    Text(c.name, fontWeight = FontWeight.SemiBold)
                                    Text(
                                        c.phone ?: "-",
                                        style = MaterialTheme.typography.bodySmall,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    )
                                }
                            }
                            IconButton(onClick = { toDelete = c }) {
                                Icon(Icons.Default.Delete, "Hapus", tint = MaterialTheme.colorScheme.error)
                            }
                        }
                    }
                }
                if (items.isEmpty()) {
                    item {
                        Text("Belum ada pelanggan.", color = MaterialTheme.colorScheme.onSurfaceVariant,
                            modifier = Modifier.padding(16.dp))
                    }
                }
            }
        }
        FloatingActionButton(
            onClick = { adding = true },
            modifier = Modifier.align(Alignment.BottomEnd).padding(16.dp),
        ) { Icon(Icons.Default.Add, "Tambah pelanggan") }
    }

    if (adding || editing != null) {
        CustomerSheet(
            initial = editing,
            onSave = { input ->
                scope.launch {
                    runCatching { api.saveCustomer(input) }
                        .onSuccess { reload() }
                        .onFailure { message = it.message }
                }
                adding = false; editing = null
            },
            onDismiss = { adding = false; editing = null },
            onError = { message = it },
        )
    }
    toDelete?.let { c ->
        ConfirmDialog(
            title = "Hapus pelanggan?", text = "\"${c.name}\" akan dihapus.",
            confirmLabel = "Hapus", destructive = true,
            onConfirm = {
                scope.launch {
                    runCatching { api.deleteCustomer(c.id) }.onSuccess { reload() }
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
private fun CustomerSheet(
    initial: Customer?,
    onSave: (CustomerInput) -> Unit,
    onDismiss: () -> Unit,
    onError: (String) -> Unit,
) {
    var name by remember { mutableStateOf(initial?.name ?: "") }
    var phone by remember { mutableStateOf(initial?.phone ?: "") }
    var email by remember { mutableStateOf(initial?.email ?: "") }
    var address by remember { mutableStateOf(initial?.address ?: "") }
    var note by remember { mutableStateOf(initial?.note ?: "") }

    ModalBottomSheet(onDismissRequest = onDismiss) {
        Column(Modifier.fillMaxWidth().padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
            Text(
                if (initial == null) "Tambah Pelanggan" else "Ubah Pelanggan",
                style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold,
            )
            FormField("No. HP (wajib)", phone, { phone = it }, keyboard = KeyboardType.Phone)
            FormField("Nama", name, { name = it })
            FormField("Email", email, { email = it }, keyboard = KeyboardType.Email)
            FormField("Alamat", address, { address = it })
            FormField("Catatan", note, { note = it })
            Button(
                onClick = {
                    when {
                        phone.isBlank() -> onError("Nomor HP wajib diisi (dipakai verifikasi di kasir).")
                        name.isBlank() -> onError("Nama pelanggan wajib diisi.")
                        else -> onSave(
                            CustomerInput(
                                id = initial?.id, name = name.trim(), phone = phone.trim(),
                                email = email.trim().ifBlank { null },
                                address = address.trim().ifBlank { null },
                                note = note.trim().ifBlank { null },
                                is_active = initial?.is_active ?: true,
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
