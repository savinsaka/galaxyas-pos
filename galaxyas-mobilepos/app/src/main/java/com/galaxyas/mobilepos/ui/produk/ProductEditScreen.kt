package com.galaxyas.mobilepos.ui.produk

import androidx.compose.foundation.horizontalScroll
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
import androidx.compose.material.icons.filled.QrCodeScanner
import androidx.compose.material3.Button
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Switch
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
import androidx.compose.ui.unit.dp
import com.galaxyas.mobilepos.data.model.Brand
import com.galaxyas.mobilepos.data.model.ProductInput
import com.galaxyas.mobilepos.data.model.ProductWithStock
import com.galaxyas.mobilepos.data.network.ApiClient
import com.galaxyas.mobilepos.scanner.ScannerScreen
import com.galaxyas.mobilepos.ui.common.ConfirmDialog
import com.galaxyas.mobilepos.ui.common.FormField
import com.galaxyas.mobilepos.ui.common.MessageDialog
import com.galaxyas.mobilepos.ui.common.NumberField
import com.galaxyas.mobilepos.ui.common.SectionCard
import com.galaxyas.mobilepos.util.formatQty
import kotlinx.coroutines.launch

/**
 * Tambah/Edit Barang. `initial` null = barang baru (diisi layar daftar lewat
 * buffer di AppContainer supaya tidak perlu refetch). Barcode bisa diisi lewat
 * scan kamera (mode sekali).
 */
@Composable
fun ProductEditScreen(
    api: ApiClient,
    initial: ProductWithStock?,
    onSaved: () -> Unit,
) {
    val scope = rememberCoroutineScope()
    val isNew = initial == null

    var brands by remember { mutableStateOf<List<Brand>>(emptyList()) }
    var name by remember { mutableStateOf(initial?.name ?: "") }
    var barcode by remember { mutableStateOf(initial?.barcode ?: "") }
    var brand by remember { mutableStateOf(initial?.brand ?: "") }
    var category by remember { mutableStateOf(initial?.category ?: "") }
    var unit by remember { mutableStateOf(initial?.unit ?: "") }
    var sellPrice by remember { mutableStateOf(initial?.sell_price?.toLong()?.toString() ?: "") }
    var costPrice by remember { mutableStateOf(initial?.cost_price?.toLong()?.toString() ?: "") }
    var defaultDiscount by remember { mutableStateOf(initial?.default_discount?.toLong()?.toString() ?: "") }
    var isActive by remember { mutableStateOf(initial?.is_active ?: true) }
    var stockText by remember { mutableStateOf("") }

    var scanning by remember { mutableStateOf(false) }
    var busy by remember { mutableStateOf(false) }
    var message by remember { mutableStateOf<String?>(null) }
    var confirmDelete by remember { mutableStateOf(false) }

    LaunchedEffect(Unit) {
        brands = runCatching { api.listBrands() }.getOrDefault(emptyList())
    }

    if (scanning) {
        ScannerScreen(
            continuous = false,
            onResult = { barcode = it; scanning = false },
            onClose = { scanning = false },
        )
        return
    }

    fun save() {
        if (name.isBlank()) { message = "Nama barang wajib diisi."; return }
        busy = true
        scope.launch {
            try {
                val saved = api.saveProduct(
                    ProductInput(
                        id = initial?.id,
                        name = name.trim(),
                        barcode = barcode.trim().ifBlank { null },
                        category = category.trim().ifBlank { null },
                        brand = brand.trim().ifBlank { null },
                        unit = unit.trim().ifBlank { null },
                        sell_price = sellPrice.toDoubleOrNull() ?: 0.0,
                        cost_price = costPrice.toDoubleOrNull() ?: 0.0,
                        default_discount = defaultDiscount.toDoubleOrNull() ?: 0.0,
                        is_active = isActive,
                    ),
                )
                // Stok awal hanya untuk barang BARU — koreksi stok lewat Opname.
                val stok = stockText.toDoubleOrNull()
                if (isNew && stok != null && stok > 0) api.setStock(saved.id, stok)
                busy = false
                onSaved()
            } catch (e: Exception) {
                busy = false
                message = e.message
            }
        }
    }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        SectionCard("Identitas") {
            FormField("Nama Barang", name, { name = it })
            FormField(
                "Barcode", barcode, { barcode = it },
                trailing = {
                    IconButton(onClick = { scanning = true }) {
                        Icon(Icons.Default.QrCodeScanner, contentDescription = "Scan barcode")
                    }
                },
            )
            FormField("Kategori", category, { category = it })
            FormField("Satuan", unit, { unit = it })
        }

        SectionCard("Merek") {
            FormField("Merek", brand, { brand = it })
            if (brands.isNotEmpty()) {
                Row(
                    Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                ) {
                    brands.take(15).forEach { b ->
                        FilterChip(
                            selected = brand == b.name,
                            onClick = { brand = b.name },
                            label = { Text(b.name) },
                        )
                    }
                }
            }
        }

        SectionCard("Harga") {
            NumberField("Harga Jual (Rp)", sellPrice, { sellPrice = it })
            NumberField("Harga Pokok (Rp)", costPrice, { costPrice = it })
            NumberField("Diskon Default per pcs (Rp)", defaultDiscount, { defaultDiscount = it })
        }

        SectionCard("Stok & Status") {
            if (isNew) {
                NumberField("Stok Awal", stockText, { stockText = it }, decimal = true)
            } else {
                Text(
                    "Stok saat ini: ${formatQty(initial!!.stock_qty)} — ubah lewat Opname atau Item Masuk/Keluar.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text("Barang aktif", Modifier.weight(1f))
                Switch(checked = isActive, onCheckedChange = { isActive = it })
            }
        }

        Row {
            if (!isNew) {
                OutlinedButton(onClick = { confirmDelete = true }, enabled = !busy) {
                    Text("Hapus", color = MaterialTheme.colorScheme.error)
                }
                Spacer(Modifier.width(8.dp))
            }
            Button(onClick = { save() }, enabled = !busy, modifier = Modifier.weight(1f)) {
                Text(if (busy) "Menyimpan…" else "Simpan")
            }
        }
        Spacer(Modifier.width(8.dp))
    }

    message?.let { MessageDialog(it) { message = null } }
    if (confirmDelete && initial != null) {
        ConfirmDialog(
            title = "Hapus barang?",
            text = "\"$name\" akan dihapus dari daftar barang.",
            confirmLabel = "Hapus",
            destructive = true,
            onConfirm = {
                scope.launch {
                    runCatching { api.deleteProduct(initial.id) }
                        .onSuccess { onSaved() }
                        .onFailure { message = it.message }
                }
            },
            onDismiss = { confirmDelete = false },
        )
    }
}
