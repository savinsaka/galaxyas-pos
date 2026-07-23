package com.galaxyas.mobilepos.ui.menu

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
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
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.RadioButton
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
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.galaxyas.mobilepos.data.SettingsRepository
import com.galaxyas.mobilepos.printer.BondedPrinter
import com.galaxyas.mobilepos.printer.BtPrinter
import kotlinx.coroutines.launch

/**
 * Pengaturan Printer (P2): pilih printer Bluetooth yang sudah dipasangkan +
 * lebar kertas + test print. Printer disimpan di setting key `receipt_printer`
 * (MAC) — sama seperti desktop, jadi jalur cetak struk/kasir memakainya.
 */
@Composable
fun SettingsPrinterScreen(settings: SettingsRepository) {
    val context = LocalContext.current
    val all by settings.settings.collectAsState()
    val scope = rememberCoroutineScope()

    var granted by remember { mutableStateOf(BtPrinter.hasPermission(context)) }
    var devices by remember { mutableStateOf<List<BondedPrinter>>(emptyList()) }
    var status by remember { mutableStateOf<String?>(null) }
    var testing by remember { mutableStateOf(false) }

    val selectedMac = all["receipt_printer"]
    val paper = if (all["receipt_paper"] == "58") "58" else "80"

    val permLauncher = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) {
        granted = it
        if (it) devices = BtPrinter.bondedPrinters(context)
    }

    fun refresh() {
        if (BtPrinter.hasPermission(context)) {
            granted = true
            devices = BtPrinter.bondedPrinters(context)
        } else {
            BtPrinter.requiredPermission()?.let { permLauncher.launch(it) }
        }
    }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text("Printer Bluetooth", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold)
        Text(
            "Pasangkan printer thermal dulu lewat Pengaturan Bluetooth HP, lalu pilih di sini.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )

        OutlinedButton(onClick = { refresh() }, modifier = Modifier.fillMaxWidth()) {
            Text(if (granted) "Muat Ulang Daftar Printer" else "Izinkan Bluetooth & Muat Printer")
        }

        if (devices.isEmpty() && granted) {
            Text("Belum ada perangkat terpasang.", color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        devices.forEach { d ->
            Card(Modifier.fillMaxWidth()) {
                Row(
                    Modifier.fillMaxWidth().padding(horizontal = 12.dp, vertical = 4.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    RadioButton(
                        selected = selectedMac == d.mac,
                        onClick = {
                            scope.launch {
                                settings.set("receipt_printer", d.mac)
                                settings.set("bt_printer_name", d.name)
                            }
                        },
                    )
                    Column(Modifier.weight(1f)) {
                        Text(d.name, fontWeight = FontWeight.SemiBold)
                        Text(d.mac, style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                }
            }
        }

        Text("Lebar Kertas", fontWeight = FontWeight.SemiBold, modifier = Modifier.padding(top = 8.dp))
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            listOf("58", "80").forEach { p ->
                FilterChip(
                    selected = paper == p,
                    onClick = { scope.launch { settings.set("receipt_paper", p) } },
                    label = { Text("$p mm") },
                )
            }
        }

        Button(
            onClick = {
                val mac = selectedMac
                if (mac.isNullOrBlank()) { status = "Pilih printer dulu."; return@Button }
                testing = true
                status = null
                scope.launch {
                    val bytes = testTicket(all["store_name"] ?: "GALAXYAS POS", paper)
                    val res = BtPrinter.print(context, mac, bytes)
                    testing = false
                    status = res.fold({ "Test print terkirim." }, { it.message ?: "Gagal." })
                }
            },
            enabled = !testing && selectedMac != null,
            modifier = Modifier.fillMaxWidth(),
        ) { Text(if (testing) "Mencetak…" else "🖨️ Test Print") }

        status?.let { Text(it, color = MaterialTheme.colorScheme.onSurfaceVariant) }
    }
}

/** Struk test kecil (tanpa perlu transaksi). */
private fun testTicket(storeName: String, paper: String): ByteArray {
    val w = if (paper == "58") 32 else 48
    val esc = 0x1b
    val gs = 0x1d
    val out = java.io.ByteArrayOutputStream()
    fun raw(vararg b: Int) = b.forEach { out.write(it) }
    fun line(s: String) {
        for (c in s) out.write(if (c.code < 128) c.code else 63)
        out.write(0x0a)
    }
    raw(esc, 0x40) // init
    raw(esc, 0x61, 1) // center
    raw(esc, 0x45, 1); raw(gs, 0x21, 0x01)
    line(storeName)
    raw(gs, 0x21, 0x00); raw(esc, 0x45, 0)
    line("TEST PRINT OK")
    line("-".repeat(w))
    line("Printer siap dipakai.")
    line("Lebar kertas: $paper mm")
    raw(esc, 0x64, 3) // feed
    raw(gs, 0x56, 1) // cut
    return out.toByteArray()
}
