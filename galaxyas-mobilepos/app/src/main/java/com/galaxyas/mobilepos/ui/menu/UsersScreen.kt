package com.galaxyas.mobilepos.ui.menu

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
import androidx.compose.material3.Checkbox
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
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
import com.galaxyas.mobilepos.data.Session
import com.galaxyas.mobilepos.data.model.User
import com.galaxyas.mobilepos.data.model.UserInput
import com.galaxyas.mobilepos.data.network.ApiClient
import com.galaxyas.mobilepos.ui.common.ConfirmDialog
import com.galaxyas.mobilepos.ui.common.FormField
import com.galaxyas.mobilepos.ui.common.MessageDialog
import kotlinx.coroutines.launch

private val MODULES = listOf(
    "master" to "Master Data",
    "penjualan" to "Penjualan",
    "persediaan" to "Persediaan",
    "laporan" to "Laporan",
    "pengaturan" to "Pengaturan",
)

/** Hak Akses: kelola pengguna + izin modul. Hanya admin. */
@Composable
fun UsersScreen(api: ApiClient, session: Session) {
    val scope = rememberCoroutineScope()
    var items by remember { mutableStateOf<List<User>>(emptyList()) }
    var editing by remember { mutableStateOf<User?>(null) }
    var adding by remember { mutableStateOf(false) }
    var toDelete by remember { mutableStateOf<User?>(null) }
    var message by remember { mutableStateOf<String?>(null) }

    val isAdmin = session.user.value?.role == "admin"

    suspend fun reload() {
        runCatching { api.listUsers() }
            .onSuccess { items = it }
            .onFailure { message = it.message }
    }
    LaunchedEffect(Unit) { if (isAdmin) reload() }

    if (!isAdmin) {
        Text(
            "Hanya admin yang bisa mengelola hak akses.",
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(24.dp),
        )
        return
    }

    Box(Modifier.fillMaxSize()) {
        LazyColumn(
            Modifier.fillMaxSize().padding(horizontal = 12.dp),
            verticalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            item { Spacer(Modifier.width(4.dp)) }
            items(items, key = { it.id }) { u ->
                Card(Modifier.fillMaxWidth()) {
                    Row(Modifier.fillMaxWidth().padding(start = 12.dp), verticalAlignment = Alignment.CenterVertically) {
                        TextButton(onClick = { editing = u }, modifier = Modifier.weight(1f)) {
                            Column(Modifier.fillMaxWidth()) {
                                Text("${u.name} (${u.username})", fontWeight = FontWeight.SemiBold)
                                Text(
                                    if (u.role == "admin") "admin — semua akses"
                                    else u.permissions.joinToString(", ").ifBlank { "tanpa akses modul" },
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                        }
                        IconButton(onClick = { toDelete = u }) {
                            Icon(Icons.Default.Delete, "Hapus", tint = MaterialTheme.colorScheme.error)
                        }
                    }
                }
            }
        }
        FloatingActionButton(
            onClick = { adding = true },
            modifier = Modifier.align(Alignment.BottomEnd).padding(16.dp),
        ) { Icon(Icons.Default.Add, "Tambah pengguna") }
    }

    if (adding || editing != null) {
        UserSheet(
            initial = editing,
            onSave = { input ->
                scope.launch {
                    runCatching { api.saveUser(input) }
                        .onSuccess { reload() }
                        .onFailure { message = it.message }
                }
                adding = false; editing = null
            },
            onDismiss = { adding = false; editing = null },
            onError = { message = it },
        )
    }
    toDelete?.let { u ->
        ConfirmDialog(
            title = "Hapus pengguna?", text = "\"${u.name}\" tidak akan bisa login lagi.",
            confirmLabel = "Hapus", destructive = true,
            onConfirm = {
                scope.launch {
                    runCatching { api.deleteUser(u.id) }.onSuccess { reload() }
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
private fun UserSheet(
    initial: User?,
    onSave: (UserInput) -> Unit,
    onDismiss: () -> Unit,
    onError: (String) -> Unit,
) {
    var username by remember { mutableStateOf(initial?.username ?: "") }
    var name by remember { mutableStateOf(initial?.name ?: "") }
    var role by remember { mutableStateOf(initial?.role ?: "kasir") }
    var perms by remember { mutableStateOf(initial?.permissions?.toSet() ?: emptySet()) }
    var pin by remember { mutableStateOf("") }

    ModalBottomSheet(onDismissRequest = onDismiss) {
        Column(Modifier.fillMaxWidth().padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
            Text(
                if (initial == null) "Tambah Pengguna" else "Ubah Pengguna",
                style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold,
            )
            FormField("Username", username, { username = it }, enabled = initial == null)
            FormField("Nama", name, { name = it })
            FormField(
                if (initial == null) "PIN" else "PIN baru (kosongkan bila tidak diubah)",
                pin, { pin = it.filter { c -> c.isDigit() } },
                keyboard = KeyboardType.NumberPassword,
            )

            Text("Peran", style = MaterialTheme.typography.labelMedium)
            Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                FilterChip(role == "admin", { role = "admin" }, { Text("Admin") })
                FilterChip(role == "kasir", { role = "kasir" }, { Text("Kasir") })
            }

            if (role != "admin") {
                Text("Akses Modul", style = MaterialTheme.typography.labelMedium)
                MODULES.forEach { (key, label) ->
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Checkbox(
                            checked = key in perms,
                            onCheckedChange = { perms = if (key in perms) perms - key else perms + key },
                        )
                        Text(label)
                    }
                }
            } else {
                Text(
                    "Admin otomatis punya semua akses.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }

            Button(
                onClick = {
                    when {
                        username.isBlank() -> onError("Username wajib diisi.")
                        name.isBlank() -> onError("Nama wajib diisi.")
                        initial == null && pin.isBlank() -> onError("PIN wajib diisi untuk pengguna baru.")
                        else -> onSave(
                            UserInput(
                                id = initial?.id,
                                username = username.trim(),
                                name = name.trim(),
                                role = role,
                                permissions = if (role == "admin") MODULES.map { it.first } else perms.toList(),
                                pin = pin.ifBlank { null },
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
