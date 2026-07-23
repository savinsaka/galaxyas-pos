package com.galaxyas.mobilepos.ui.produk

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.ExperimentalLayoutApi
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
import androidx.compose.material3.FilterChip
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Switch
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
import com.galaxyas.mobilepos.data.model.DiscountPeriod
import com.galaxyas.mobilepos.data.model.DiscountPeriodInput
import com.galaxyas.mobilepos.data.network.ApiClient
import com.galaxyas.mobilepos.ui.common.ConfirmDialog
import com.galaxyas.mobilepos.ui.common.FormField
import com.galaxyas.mobilepos.ui.common.MessageDialog
import com.galaxyas.mobilepos.ui.common.NumberField
import com.galaxyas.mobilepos.util.formatIDR
import kotlinx.coroutines.launch

private val DAYS = listOf(
    "min" to "Min", "sen" to "Sen", "sel" to "Sel", "rab" to "Rab",
    "kam" to "Kam", "jum" to "Jum", "sab" to "Sab",
)

private fun daysLabel(d: String): String =
    if (d == "everyday") "Setiap hari"
    else d.split(",").joinToString(", ") { k -> DAYS.firstOrNull { it.first == k }?.second ?: k }

/** Diskon Periodik: aturan diskon per barang/merek dengan jadwal hari. */
@Composable
fun DiscountsScreen(api: ApiClient) {
    val scope = rememberCoroutineScope()
    var items by remember { mutableStateOf<List<DiscountPeriod>>(emptyList()) }
    var editing by remember { mutableStateOf<DiscountPeriod?>(null) }
    var adding by remember { mutableStateOf(false) }
    var toDelete by remember { mutableStateOf<DiscountPeriod?>(null) }
    var message by remember { mutableStateOf<String?>(null) }

    suspend fun reload() {
        runCatching { api.listDiscounts() }
            .onSuccess { items = it }
            .onFailure { message = it.message }
    }
    LaunchedEffect(Unit) { reload() }

    Box(Modifier.fillMaxSize()) {
        LazyColumn(
            Modifier.fillMaxSize().padding(horizontal = 12.dp),
            verticalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            item { Spacer(Modifier.width(4.dp)) }
            items(items, key = { it.id }) { d ->
                Card(Modifier.fillMaxWidth()) {
                    Row(Modifier.fillMaxWidth().padding(start = 12.dp), verticalAlignment = Alignment.CenterVertically) {
                        TextButton(onClick = { editing = d }, modifier = Modifier.weight(1f)) {
                            Column(Modifier.fillMaxWidth()) {
                                Row {
                                    Text(
                                        d.target_label ?: d.target,
                                        fontWeight = FontWeight.SemiBold,
                                        modifier = Modifier.weight(1f),
                                    )
                                    Text(
                                        if (d.discount_type == "percent") "${d.value.toLong()}%"
                                        else formatIDR(d.value),
                                        fontWeight = FontWeight.Bold,
                                    )
                                }
                                Text(
                                    "${if (d.scope == "item") "Barang" else "Merek"} · ${daysLabel(d.days)}" +
                                        if (!d.is_active) " · nonaktif" else "",
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                        }
                        IconButton(onClick = { toDelete = d }) {
                            Icon(Icons.Default.Delete, "Hapus", tint = MaterialTheme.colorScheme.error)
                        }
                    }
                }
            }
            if (items.isEmpty()) {
                item {
                    Text("Belum ada diskon periodik.", color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(16.dp))
                }
            }
        }
        FloatingActionButton(
            onClick = { adding = true },
            modifier = Modifier.align(Alignment.BottomEnd).padding(16.dp),
        ) { Icon(Icons.Default.Add, "Tambah diskon") }
    }

    if (adding || editing != null) {
        DiscountSheet(
            initial = editing,
            onSave = { input ->
                scope.launch {
                    runCatching { api.saveDiscount(input) }
                        .onSuccess { reload() }
                        .onFailure { message = it.message }
                }
                adding = false; editing = null
            },
            onDismiss = { adding = false; editing = null },
            onError = { message = it },
        )
    }
    toDelete?.let { d ->
        ConfirmDialog(
            title = "Hapus diskon?", text = "Aturan diskon untuk \"${d.target_label ?: d.target}\" akan dihapus.",
            confirmLabel = "Hapus", destructive = true,
            onConfirm = {
                scope.launch {
                    runCatching { api.deleteDiscount(d.id) }.onSuccess { reload() }
                        .onFailure { message = it.message }
                }
            },
            onDismiss = { toDelete = null },
        )
    }
    message?.let { MessageDialog(it) { message = null } }
}

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
private fun DiscountSheet(
    initial: DiscountPeriod?,
    onSave: (DiscountPeriodInput) -> Unit,
    onDismiss: () -> Unit,
    onError: (String) -> Unit,
) {
    var code by remember { mutableStateOf(initial?.code ?: "") }
    var scopeVal by remember { mutableStateOf(initial?.scope ?: "item") }
    var target by remember { mutableStateOf(initial?.target ?: "") }
    var targetLabel by remember { mutableStateOf(initial?.target_label ?: "") }
    var type by remember { mutableStateOf(initial?.discount_type ?: "amount") }
    var value by remember { mutableStateOf(initial?.value?.toLong()?.toString() ?: "") }
    var everyday by remember { mutableStateOf((initial?.days ?: "everyday") == "everyday") }
    var dayset by remember {
        mutableStateOf(
            (initial?.days ?: "").split(",").filter { it.isNotBlank() }.toSet(),
        )
    }
    var isActive by remember { mutableStateOf(initial?.is_active ?: true) }
    var priority by remember { mutableStateOf((initial?.priority ?: 1L).toString()) }

    ModalBottomSheet(onDismissRequest = onDismiss) {
        Column(
            Modifier.fillMaxWidth().padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            Text(
                if (initial == null) "Tambah Diskon" else "Ubah Diskon",
                style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold,
            )
            FormField("Kode", code, { code = it })

            Text("Berlaku untuk", style = MaterialTheme.typography.labelMedium)
            Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                FilterChip(scopeVal == "item", { scopeVal = "item" }, { Text("Barang") })
                FilterChip(scopeVal == "brand", { scopeVal = "brand" }, { Text("Merek") })
            }
            FormField(
                if (scopeVal == "item") "ID Barang (product_id)" else "Nama Merek",
                target, { target = it },
            )
            FormField("Label tampilan (opsional)", targetLabel, { targetLabel = it })

            Text("Jenis diskon", style = MaterialTheme.typography.labelMedium)
            Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                FilterChip(type == "amount", { type = "amount" }, { Text("Rupiah") })
                FilterChip(type == "percent", { type = "percent" }, { Text("Persen") })
            }
            NumberField(if (type == "percent") "Nilai (%)" else "Nilai (Rp)", value, { value = it })

            Row(verticalAlignment = Alignment.CenterVertically) {
                Text("Berlaku setiap hari", Modifier.weight(1f))
                Switch(checked = everyday, onCheckedChange = { everyday = it })
            }
            if (!everyday) {
                FlowRow(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    DAYS.forEach { (k, label) ->
                        FilterChip(
                            selected = k in dayset,
                            onClick = { dayset = if (k in dayset) dayset - k else dayset + k },
                            label = { Text(label) },
                        )
                    }
                }
            }

            NumberField("Prioritas", priority, { priority = it })
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text("Aktif", Modifier.weight(1f))
                Switch(checked = isActive, onCheckedChange = { isActive = it })
            }

            Button(
                onClick = {
                    val days = if (everyday) "everyday" else dayset.joinToString(",")
                    when {
                        target.isBlank() -> onError("Target (barang/merek) wajib diisi.")
                        !everyday && dayset.isEmpty() -> onError("Pilih minimal satu hari.")
                        else -> onSave(
                            DiscountPeriodInput(
                                id = initial?.id,
                                code = code.trim().ifBlank { "DISC" },
                                scope = scopeVal,
                                target = target.trim(),
                                target_label = targetLabel.trim().ifBlank { null },
                                discount_type = type,
                                value = value.toDoubleOrNull() ?: 0.0,
                                days = days,
                                is_active = isActive,
                                priority = priority.toLongOrNull() ?: 1L,
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
