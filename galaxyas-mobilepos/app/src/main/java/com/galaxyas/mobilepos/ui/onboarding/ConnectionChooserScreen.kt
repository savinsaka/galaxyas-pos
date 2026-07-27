package com.galaxyas.mobilepos.ui.onboarding

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
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
import androidx.compose.ui.unit.dp
import com.galaxyas.mobilepos.data.SavedServer
import com.galaxyas.mobilepos.data.ServerRegistry
import com.galaxyas.mobilepos.data.network.ConnMode
import com.galaxyas.mobilepos.data.network.RpcClient
import kotlinx.coroutines.launch

/**
 * Layar pilih jalur koneksi, tampil setiap kali app dibuka walau server sudah
 * pernah dipakai — permintaan pemilik toko: kasir yang tahu situasinya (di toko
 * atau di luar) yang memilih, bukan aplikasi yang menebak.
 *
 * LOCAL dipakai saat HP di wifi toko: paling cepat, tidak makan kuota, dan tetap
 * jalan walau internet toko mati. ONLINE dipakai dari mana saja selama PC kasir
 * menyala.
 */
@Composable
fun ConnectionChooserScreen(
    registry: ServerRegistry,
    rpc: RpcClient,
    onChosen: () -> Unit,
    onAddServer: () -> Unit,
) {
    val servers by registry.servers.collectAsState()
    val lastMode by registry.activeMode.collectAsState()
    val scope = rememberCoroutineScope()

    var busy by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }

    /** Uji jalurnya dulu supaya salah pilih ketahuan di sini, bukan nanti di kasir. */
    fun choose(server: SavedServer, mode: ConnMode) {
        val remote = server.toRemote(mode) ?: run {
            error = "Alamat ${mode.label} belum diatur untuk server ini."
            return
        }
        busy = true
        error = null
        scope.launch {
            try {
                rpc.healthCheck(remote)
                registry.select(server.id, mode)
                onChosen()
            } catch (e: Exception) {
                error = e.message ?: RpcClient.connectErrorFor(mode)
            } finally {
                busy = false
            }
        }
    }

    Column(
        modifier = Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(24.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        Text(
            "GALAXYAS POS",
            style = MaterialTheme.typography.headlineMedium,
            fontWeight = FontWeight.ExtraBold,
            color = MaterialTheme.colorScheme.secondary,
        )
        Text("Pilih cara menyambung", color = MaterialTheme.colorScheme.onSurfaceVariant)
        Spacer(Modifier.padding(6.dp))

        Text(
            "LOCAL = HP sedang di wifi toko (paling cepat, hemat kuota).\n" +
                "ONLINE = dari mana saja, selama PC kasir menyala.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.fillMaxWidth(),
        )
        Spacer(Modifier.padding(8.dp))

        servers.forEach { server ->
            Card(modifier = Modifier.fillMaxWidth().padding(bottom = 10.dp)) {
                Column(modifier = Modifier.padding(12.dp)) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text(
                            server.name,
                            fontWeight = FontWeight.SemiBold,
                            modifier = Modifier.weight(1f),
                        )
                        IconButton(
                            enabled = !busy,
                            onClick = { scope.launch { registry.remove(server.id) } },
                        ) {
                            Icon(
                                Icons.Default.Delete,
                                contentDescription = "Hapus server",
                                tint = MaterialTheme.colorScheme.error,
                            )
                        }
                    }
                    HorizontalDivider()
                    Spacer(Modifier.padding(4.dp))
                    Row(modifier = Modifier.fillMaxWidth()) {
                        ModeButton(
                            mode = ConnMode.LOCAL,
                            server = server,
                            preferred = lastMode == ConnMode.LOCAL,
                            enabled = !busy,
                            modifier = Modifier.weight(1f),
                            onClick = { choose(server, ConnMode.LOCAL) },
                        )
                        Spacer(Modifier.width(8.dp))
                        ModeButton(
                            mode = ConnMode.ONLINE,
                            server = server,
                            preferred = lastMode == ConnMode.ONLINE,
                            enabled = !busy,
                            modifier = Modifier.weight(1f),
                            onClick = { choose(server, ConnMode.ONLINE) },
                        )
                    }
                }
            }
        }

        error?.let {
            Text(
                it,
                color = MaterialTheme.colorScheme.error,
                style = MaterialTheme.typography.bodySmall,
                modifier = Modifier.fillMaxWidth().padding(bottom = 8.dp),
            )
        }
        if (busy) {
            Text(
                "Memeriksa koneksi…",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }

        OutlinedButton(
            onClick = onAddServer,
            enabled = !busy,
            modifier = Modifier.fillMaxWidth(),
        ) { Text("+ Tambah Server") }
    }
}

@Composable
private fun ModeButton(
    mode: ConnMode,
    server: SavedServer,
    preferred: Boolean,
    enabled: Boolean,
    modifier: Modifier = Modifier,
    onClick: () -> Unit,
) {
    val supported = server.supports(mode)
    val content: @Composable () -> Unit = {
        Column(horizontalAlignment = Alignment.CenterHorizontally) {
            Text("[${mode.label}]", fontWeight = FontWeight.Bold)
            Text(
                server.addressOf(mode),
                style = MaterialTheme.typography.labelSmall,
            )
        }
    }
    // Jalur yang terakhir dipakai ditonjolkan supaya kasir yang rutinnya sama
    // tidak perlu berpikir tiap buka app.
    if (preferred && supported) {
        Button(onClick = onClick, enabled = enabled, modifier = modifier) { content() }
    } else {
        OutlinedButton(
            onClick = onClick,
            enabled = enabled && supported,
            modifier = modifier,
        ) { content() }
    }
}
