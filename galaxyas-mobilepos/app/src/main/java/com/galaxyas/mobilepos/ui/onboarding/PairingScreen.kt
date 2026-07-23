package com.galaxyas.mobilepos.ui.onboarding

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
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
import com.galaxyas.mobilepos.data.ServerRegistry
import com.galaxyas.mobilepos.data.network.RpcClient
import kotlinx.coroutines.launch

/**
 * Onboarding pairing Server Pusat (padanan ServerPicker desktop, tanpa entry
 * "Server Lokal" — HP selalu client). Validasi kode via GET /health SEBELUM
 * disimpan, persis alur "+ Tambah Server" desktop.
 */
@Composable
fun PairingScreen(
    registry: ServerRegistry,
    rpc: RpcClient,
    onPaired: () -> Unit,
) {
    val servers by registry.servers.collectAsState()
    val active by registry.activeServer.collectAsState()
    val scope = rememberCoroutineScope()

    var name by remember { mutableStateOf("") }
    var host by remember { mutableStateOf("") }
    var port by remember { mutableStateOf("8899") }
    var token by remember { mutableStateOf("") }
    var busy by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    var adding by remember { mutableStateOf(false) }

    val showForm = adding || servers.isEmpty()

    fun connectNew() {
        val p = port.toIntOrNull()
        if (name.isBlank() || host.isBlank() || token.isBlank() || p == null) {
            error = "Nama, IP, Port, dan Kode Pairing wajib diisi."
            return
        }
        busy = true
        error = null
        scope.launch {
            try {
                rpc.healthCheck(host.trim(), p, token.trim().uppercase())
                registry.add(name.trim(), host.trim(), p, token.trim().uppercase())
                    .also { registry.select(it.id) }
                adding = false
                onPaired()
            } catch (e: Exception) {
                error = e.message ?: "Gagal terhubung."
            } finally {
                busy = false
            }
        }
    }

    Column(
        modifier = Modifier.fillMaxSize().verticalScroll(rememberScrollState())
            .padding(24.dp).imePadding(),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        Text(
            "GALAXYAS POS",
            style = MaterialTheme.typography.headlineMedium,
            fontWeight = FontWeight.ExtraBold,
            color = MaterialTheme.colorScheme.secondary,
        )
        Text(
            if (servers.isEmpty()) "Hubungkan ke Server Pusat" else "Pilih Server Pusat",
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(Modifier.padding(6.dp))

        if (servers.isEmpty()) {
            Text(
                "Aplikasi ini adalah kasir tambahan — semua data (barang, stok, transaksi) " +
                    "tersimpan di PC yang menjalankan GALAXYAS POS sebagai Server Pusat. " +
                    "Pastikan HP dan PC berada di jaringan wifi yang sama.\n\n" +
                    "Di PC: Pengaturan → Server Pusat → centang \"Jadikan PC ini Server Pusat\", " +
                    "lalu masukkan IP, Port, dan Kode Pairing yang tampil.",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(Modifier.padding(6.dp))
        }

        if (servers.isNotEmpty()) {
            Card(modifier = Modifier.fillMaxWidth()) {
                Column {
                    servers.forEachIndexed { i, s ->
                        if (i > 0) HorizontalDivider()
                        Row(
                            modifier = Modifier.fillMaxWidth().padding(start = 12.dp),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Column(
                                modifier = Modifier.weight(1f).padding(vertical = 10.dp),
                            ) {
                                Text(s.name, fontWeight = FontWeight.SemiBold)
                                Text(
                                    "${s.host}:${s.port}" + if (active?.id == s.id) " · aktif" else "",
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                            OutlinedButton(
                                enabled = !busy,
                                onClick = {
                                    scope.launch {
                                        registry.select(s.id)
                                        onPaired()
                                    }
                                },
                            ) { Text("Pilih") }
                            IconButton(
                                enabled = !busy,
                                onClick = { scope.launch { registry.remove(s.id) } },
                            ) {
                                Icon(
                                    Icons.Default.Delete,
                                    contentDescription = "Hapus server",
                                    tint = MaterialTheme.colorScheme.error,
                                )
                            }
                        }
                    }
                }
            }
            Spacer(Modifier.padding(6.dp))
        }

        if (showForm) {
            OutlinedTextField(
                value = name, onValueChange = { name = it },
                label = { Text("Nama Server") }, placeholder = { Text("Kasir Pusat") },
                singleLine = true, enabled = !busy, modifier = Modifier.fillMaxWidth(),
            )
            OutlinedTextField(
                value = host, onValueChange = { host = it },
                label = { Text("IP Server Pusat") }, placeholder = { Text("192.168.1.10") },
                singleLine = true, enabled = !busy, modifier = Modifier.fillMaxWidth(),
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
            )
            OutlinedTextField(
                value = port, onValueChange = { port = it },
                label = { Text("Port") },
                singleLine = true, enabled = !busy, modifier = Modifier.fillMaxWidth(),
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
            )
            OutlinedTextField(
                value = token, onValueChange = { token = it.uppercase() },
                label = { Text("Kode Pairing") }, placeholder = { Text("6 karakter, lihat di PC") },
                singleLine = true, enabled = !busy, modifier = Modifier.fillMaxWidth(),
            )
            error?.let {
                Text(
                    it,
                    color = MaterialTheme.colorScheme.error,
                    style = MaterialTheme.typography.bodySmall,
                    modifier = Modifier.padding(top = 6.dp),
                )
            }
            Spacer(Modifier.padding(4.dp))
            Row(modifier = Modifier.fillMaxWidth()) {
                Button(
                    onClick = { connectNew() },
                    enabled = !busy,
                    modifier = Modifier.weight(1f),
                ) { Text(if (busy) "Menghubungkan…" else "Hubungkan") }
                if (servers.isNotEmpty()) {
                    Spacer(Modifier.width(8.dp))
                    OutlinedButton(onClick = { adding = false }, enabled = !busy) { Text("Batal") }
                }
            }
        } else {
            OutlinedButton(
                onClick = { adding = true },
                modifier = Modifier.fillMaxWidth(),
            ) { Text("+ Tambah Server") }
        }
    }
}
