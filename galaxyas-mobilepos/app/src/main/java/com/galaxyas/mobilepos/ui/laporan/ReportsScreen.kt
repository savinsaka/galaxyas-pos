package com.galaxyas.mobilepos.ui.laporan

import android.content.Context
import androidx.compose.foundation.horizontalScroll
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
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.galaxyas.mobilepos.data.SettingsRepository
import com.galaxyas.mobilepos.data.network.ApiClient
import com.galaxyas.mobilepos.printer.BtPrinter
import com.galaxyas.mobilepos.printer.ReportEscPosDoc
import com.galaxyas.mobilepos.printer.ReportEscPosRow
import com.galaxyas.mobilepos.printer.ReportEscPosSection
import com.galaxyas.mobilepos.printer.buildReportEscPos
import com.galaxyas.mobilepos.printer.parseReceiptConfig
import com.galaxyas.mobilepos.ui.common.FormField
import com.galaxyas.mobilepos.ui.common.MessageDialog
import com.galaxyas.mobilepos.util.formatIDR
import com.galaxyas.mobilepos.util.formatQty
import com.galaxyas.mobilepos.util.todayIso
import java.util.Calendar
import java.text.SimpleDateFormat
import java.util.Locale
import kotlinx.coroutines.launch

private enum class ReportKind(val label: String, val title: String) {
    PRODUCT("Per Barang", "LAPORAN PER BARANG"),
    BRAND("Per Merek", "LAPORAN PER MEREK"),
    DETAIL("Detail Item", "LAPORAN DETAIL ITEM"),
    DAILY("Harian", "LAPORAN HARIAN"),
}

/** Satu baris hasil laporan yang sudah dinormalisasi untuk tampilan & cetak. */
private data class Line(
    val title: String,
    val sub: String?,
    val qty: Double,
    val net: Double,
    val gross: Double = 0.0,
    val discount: Double = 0.0,
)

private fun daysAgoIso(days: Int): String {
    val cal = Calendar.getInstance()
    cal.add(Calendar.DAY_OF_YEAR, -days)
    return SimpleDateFormat("yyyy-MM-dd", Locale.US).format(cal.time)
}

private fun monthStartIso(): String {
    val cal = Calendar.getInstance()
    cal.set(Calendar.DAY_OF_MONTH, 1)
    return SimpleDateFormat("yyyy-MM-dd", Locale.US).format(cal.time)
}

/**
 * Hub laporan: 4 jenis laporan berbagi filter periode + merek, ringkasan kartu,
 * daftar baris, dan tombol "Cetak sebagai struk" (buildReportEscPos ke printer BT).
 */
@Composable
fun ReportsScreen(api: ApiClient, settings: SettingsRepository) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()

    var kind by remember { mutableStateOf(ReportKind.PRODUCT) }
    var from by remember { mutableStateOf(monthStartIso()) }
    var to by remember { mutableStateOf(todayIso()) }
    var brands by remember { mutableStateOf<List<String>>(emptyList()) }
    var selectedBrands by remember { mutableStateOf<Set<String>>(emptySet()) }
    var lines by remember { mutableStateOf<List<Line>>(emptyList()) }
    var loading by remember { mutableStateOf(false) }
    var message by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(Unit) {
        brands = runCatching { api.listBrands().map { it.name } }.getOrDefault(emptyList())
    }

    suspend fun run() {
        loading = true
        try {
            val bs = selectedBrands.toList()
            lines = when (kind) {
                ReportKind.PRODUCT -> api.productSalesReport(from, to, bs).map {
                    Line(it.name, it.brand, it.qty, it.net, it.gross, it.discount)
                }
                ReportKind.BRAND -> api.brandSalesReport(from, to, bs).map {
                    Line(it.brand, null, it.qty, it.net, it.gross, it.discount)
                }
                ReportKind.DETAIL -> api.salesItemDetailReport(from, to, bs).map {
                    Line(it.name, "${it.invoice_no} · ${it.created_at.take(10)}", it.qty, it.net, it.price * it.qty, it.discount)
                }
                ReportKind.DAILY -> api.dailySalesReport(from, to, bs).map {
                    Line(it.day, null, it.qty, it.net, it.gross, it.discount)
                }
            }
        } catch (e: Exception) {
            message = e.message
        } finally {
            loading = false
        }
    }

    LaunchedEffect(kind, from, to, selectedBrands) { run() }

    val totalQty = lines.sumOf { it.qty }
    val totalNet = lines.sumOf { it.net }
    val totalDiscount = lines.sumOf { it.discount }

    fun printReport() {
        val cfg = parseReceiptConfig(settings.settings.value)
        val mac = cfg.printer
        if (mac.isNullOrBlank()) {
            message = "Printer belum dipilih (Menu → Printer & Kertas)."
            return
        }
        val doc = ReportEscPosDoc(
            title = kind.title,
            subtitle = "$from s/d $to",
            meta = if (selectedBrands.isEmpty()) "Semua merek" else selectedBrands.joinToString(", "),
            sections = listOf(
                ReportEscPosSection(
                    heading = "Ringkasan",
                    rows = listOf(
                        ReportEscPosRow(listOf("Total Qty", formatQty(totalQty))),
                        ReportEscPosRow(listOf("Total Diskon", formatIDR(totalDiscount))),
                        ReportEscPosRow(listOf("OMZET", formatIDR(totalNet)), bold = true),
                    ),
                ),
                ReportEscPosSection(
                    heading = kind.label,
                    columns = listOf("Nama", "Qty", "Omzet"),
                    rows = lines.map {
                        ReportEscPosRow(listOf(it.title, formatQty(it.qty), formatIDR(it.net)))
                    },
                ),
            ),
        )
        scope.launch {
            val res = BtPrinter.print(context, mac, buildReportEscPos(doc, cfg))
            message = res.fold({ "Laporan dikirim ke printer." }, { it.message ?: "Gagal mencetak." })
        }
    }

    Column(Modifier.fillMaxSize().padding(horizontal = 12.dp)) {
        // Jenis laporan
        Row(
            Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(vertical = 8.dp),
            horizontalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            ReportKind.entries.forEach { k ->
                FilterChip(kind == k, { kind = k }, { Text(k.label) })
            }
        }

        // Periode
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            FormField("Dari", from, { from = it }, modifier = Modifier.weight(1f))
            FormField("Sampai", to, { to = it }, modifier = Modifier.weight(1f))
        }
        Row(
            Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(vertical = 6.dp),
            horizontalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            OutlinedButton(onClick = { from = todayIso(); to = todayIso() }) { Text("Hari ini") }
            OutlinedButton(onClick = { from = daysAgoIso(6); to = todayIso() }) { Text("7 hari") }
            OutlinedButton(onClick = { from = monthStartIso(); to = todayIso() }) { Text("Bulan ini") }
        }

        // Filter merek
        if (brands.isNotEmpty()) {
            Row(
                Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(bottom = 6.dp),
                horizontalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                FilterChip(
                    selected = selectedBrands.isEmpty(),
                    onClick = { selectedBrands = emptySet() },
                    label = { Text("Semua merek") },
                )
                brands.forEach { b ->
                    FilterChip(
                        selected = b in selectedBrands,
                        onClick = {
                            selectedBrands = if (b in selectedBrands) selectedBrands - b else selectedBrands + b
                        },
                        label = { Text(b) },
                    )
                }
            }
        }

        // Ringkasan
        Card(Modifier.fillMaxWidth()) {
            Column(Modifier.padding(12.dp)) {
                Row {
                    Text("Omzet", Modifier.weight(1f), color = MaterialTheme.colorScheme.onSurfaceVariant)
                    Text(
                        formatIDR(totalNet),
                        style = MaterialTheme.typography.titleMedium,
                        fontWeight = FontWeight.ExtraBold,
                        color = MaterialTheme.colorScheme.secondary,
                    )
                }
                Row {
                    Text("Total Qty", Modifier.weight(1f), style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant)
                    Text(formatQty(totalQty), style = MaterialTheme.typography.bodySmall)
                }
                Row {
                    Text("Total Diskon", Modifier.weight(1f), style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant)
                    Text(formatIDR(totalDiscount), style = MaterialTheme.typography.bodySmall)
                }
            }
        }
        Spacer(Modifier.width(6.dp))
        Button(
            onClick = { printReport() },
            enabled = lines.isNotEmpty(),
            modifier = Modifier.fillMaxWidth().padding(vertical = 6.dp),
        ) { Text("🖨️ Cetak sebagai Struk") }

        // Baris
        LazyColumn(verticalArrangement = Arrangement.spacedBy(4.dp)) {
            items(lines) { l ->
                Column(Modifier.fillMaxWidth().padding(vertical = 6.dp)) {
                    Row {
                        Text(l.title, Modifier.weight(1f), fontWeight = FontWeight.SemiBold)
                        Text(formatIDR(l.net), fontWeight = FontWeight.Bold)
                    }
                    Row {
                        Text(
                            listOfNotNull(l.sub, "qty ${formatQty(l.qty)}").joinToString(" · "),
                            Modifier.weight(1f),
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                        if (l.discount > 0) {
                            Text(
                                "−${formatIDR(l.discount)}",
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                    }
                    HorizontalDivider()
                }
            }
            item {
                val label = when {
                    loading -> "Memuat…"
                    lines.isEmpty() -> "Tidak ada data pada periode ini."
                    else -> "${lines.size} baris."
                }
                Text(label, color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(vertical = 12.dp))
            }
        }
    }

    message?.let { MessageDialog(it) { message = null } }
}
