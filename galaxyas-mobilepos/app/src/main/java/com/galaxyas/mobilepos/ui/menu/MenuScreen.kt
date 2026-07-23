package com.galaxyas.mobilepos.ui.menu

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Card
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.galaxyas.mobilepos.data.ServerRegistry
import com.galaxyas.mobilepos.data.Session
import com.galaxyas.mobilepos.data.SettingsRepository
import com.galaxyas.mobilepos.ui.theme.APP_THEMES
import com.galaxyas.mobilepos.ui.theme.DEFAULT_THEME_KEY
import kotlinx.coroutines.launch

/**
 * Tab Menu P1: sesi + server, tema, logout / ganti server. Entri Persediaan,
 * Shift, Hak Akses, Pengaturan lengkap menyusul di P2-P4.
 */
@Composable
fun MenuScreen(
    session: Session,
    registry: ServerRegistry,
    settings: SettingsRepository,
    onChangeServer: () -> Unit,
    onOpenPrinter: () -> Unit,
    onOpen: (String) -> Unit = {},
) {
    val user by session.user.collectAsState()
    val server by registry.activeServer.collectAsState()
    val allSettings by settings.settings.collectAsState()
    val themeKey = allSettings["theme"].takeUnless { it.isNullOrBlank() } ?: DEFAULT_THEME_KEY
    val scope = rememberCoroutineScope()

    Column(
        modifier = Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(12.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Card(modifier = Modifier.fillMaxWidth()) {
            Column(modifier = Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Text("Sesi", style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant)
                Row {
                    Text("👤 ${user?.name ?: "-"}", modifier = Modifier.weight(1f))
                    Text(user?.role ?: "", color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
                Row {
                    Text("🖧 ${server?.name ?: "-"}", modifier = Modifier.weight(1f))
                    Text(
                        server?.let { "${it.host}:${it.port}" } ?: "",
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        style = MaterialTheme.typography.bodySmall,
                    )
                }
                Spacer(Modifier.padding(2.dp))
                Row {
                    OutlinedButton(
                        onClick = { session.logout() },
                        modifier = Modifier.weight(1f),
                    ) { Text("Keluar") }
                    Spacer(Modifier.width(8.dp))
                    OutlinedButton(
                        onClick = onChangeServer,
                        modifier = Modifier.weight(1f),
                    ) { Text("Ganti Server") }
                }
            }
        }

        Card(modifier = Modifier.fillMaxWidth()) {
            Column(modifier = Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                Text("Persediaan", style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant)
                listOf(
                    "opname" to "📋 Stok Opname",
                    "batch" to "📦 Item Masuk / Keluar",
                    "pengeluaran" to "💸 Pengeluaran",
                ).forEach { (route, label) ->
                    OutlinedButton(onClick = { onOpen(route) }, modifier = Modifier.fillMaxWidth()) {
                        Text(label, modifier = Modifier.fillMaxWidth())
                    }
                }
            }
        }

        Card(modifier = Modifier.fillMaxWidth()) {
            Column(modifier = Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                Text("Pengaturan", style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant)
                OutlinedButton(onClick = onOpenPrinter, modifier = Modifier.fillMaxWidth()) {
                    Text("🖨️ Printer & Kertas", modifier = Modifier.fillMaxWidth())
                }
            }
        }

        Card(modifier = Modifier.fillMaxWidth()) {
            Column(modifier = Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                Text("Tema", style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant)
                APP_THEMES.chunked(2).forEach { rowThemes ->
                    Row(modifier = Modifier.fillMaxWidth()) {
                        rowThemes.forEach { t ->
                            OutlinedButton(
                                onClick = { scope.launch { settings.set("theme", t.key) } },
                                modifier = Modifier.weight(1f).padding(end = 6.dp),
                            ) {
                                Surface(
                                    color = t.swatch,
                                    shape = CircleShape,
                                    modifier = Modifier.size(14.dp),
                                ) {}
                                Spacer(Modifier.width(6.dp))
                                Text(
                                    t.label,
                                    style = MaterialTheme.typography.bodySmall,
                                    fontWeight = if (themeKey == t.key) FontWeight.Bold else FontWeight.Normal,
                                    maxLines = 1,
                                )
                            }
                        }
                        if (rowThemes.size == 1) Spacer(Modifier.weight(1f))
                    }
                }
            }
        }

        Text(
            "GALAXYAS Mobile POS",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.fillMaxWidth().padding(top = 6.dp),
            textAlign = androidx.compose.ui.text.style.TextAlign.Center,
        )
    }
}
