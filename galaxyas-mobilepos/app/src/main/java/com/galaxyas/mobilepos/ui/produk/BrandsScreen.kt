package com.galaxyas.mobilepos.ui.produk

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Card
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
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
import com.galaxyas.mobilepos.data.model.Brand
import com.galaxyas.mobilepos.data.model.BrandInput
import com.galaxyas.mobilepos.data.network.ApiClient
import com.galaxyas.mobilepos.ui.common.ConfirmDialog
import com.galaxyas.mobilepos.ui.common.FormField
import com.galaxyas.mobilepos.ui.common.MessageDialog
import kotlinx.coroutines.launch

/** Daftar Merek: list + tambah/edit lewat dialog + hapus. */
@Composable
fun BrandsScreen(api: ApiClient) {
    val scope = rememberCoroutineScope()
    var items by remember { mutableStateOf<List<Brand>>(emptyList()) }
    var editing by remember { mutableStateOf<Brand?>(null) }
    var adding by remember { mutableStateOf(false) }
    var toDelete by remember { mutableStateOf<Brand?>(null) }
    var message by remember { mutableStateOf<String?>(null) }

    suspend fun reload() {
        runCatching { api.listBrands() }
            .onSuccess { items = it }
            .onFailure { message = it.message }
    }
    LaunchedEffect(Unit) { reload() }

    Box(Modifier.fillMaxSize()) {
        LazyColumn(
            Modifier.fillMaxSize().padding(horizontal = 12.dp),
            verticalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            item { Row(Modifier.padding(top = 8.dp)) {} }
            items(items, key = { it.id }) { b ->
                Card(Modifier.fillMaxWidth()) {
                    Row(
                        Modifier.fillMaxWidth().padding(start = 14.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        TextButton(onClick = { editing = b }, modifier = Modifier.weight(1f)) {
                            Text(b.name, Modifier.fillMaxWidth(), fontWeight = FontWeight.SemiBold)
                        }
                        IconButton(onClick = { toDelete = b }) {
                            Icon(Icons.Default.Delete, "Hapus", tint = MaterialTheme.colorScheme.error)
                        }
                    }
                }
            }
            if (items.isEmpty()) {
                item {
                    Text("Belum ada merek.", color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(16.dp))
                }
            }
        }
        FloatingActionButton(
            onClick = { adding = true },
            modifier = Modifier.align(Alignment.BottomEnd).padding(16.dp),
        ) { Icon(Icons.Default.Add, "Tambah merek") }
    }

    if (adding || editing != null) {
        val current = editing
        BrandDialog(
            initial = current?.name ?: "",
            onSave = { newName ->
                scope.launch {
                    runCatching { api.saveBrand(BrandInput(id = current?.id, name = newName.trim())) }
                        .onSuccess { reload() }
                        .onFailure { message = it.message }
                }
                adding = false; editing = null
            },
            onDismiss = { adding = false; editing = null },
        )
    }
    toDelete?.let { b ->
        ConfirmDialog(
            title = "Hapus merek?",
            text = "\"${b.name}\" akan dihapus.",
            confirmLabel = "Hapus",
            destructive = true,
            onConfirm = {
                scope.launch {
                    runCatching { api.deleteBrand(b.id) }.onSuccess { reload() }
                        .onFailure { message = it.message }
                }
            },
            onDismiss = { toDelete = null },
        )
    }
    message?.let { MessageDialog(it) { message = null } }
}

@Composable
private fun BrandDialog(initial: String, onSave: (String) -> Unit, onDismiss: () -> Unit) {
    var name by remember { mutableStateOf(initial) }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(if (initial.isBlank()) "Tambah Merek" else "Ubah Merek") },
        text = { FormField("Nama Merek", name, { name = it }) },
        confirmButton = {
            TextButton(onClick = { if (name.isNotBlank()) onSave(name) }) { Text("Simpan") }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Batal") } },
    )
}
