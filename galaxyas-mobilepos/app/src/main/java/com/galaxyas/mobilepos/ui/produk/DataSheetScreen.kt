package com.galaxyas.mobilepos.ui.produk

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.QrCodeScanner
import androidx.compose.material3.Card
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.galaxyas.mobilepos.data.model.ProductWithStock
import com.galaxyas.mobilepos.data.network.ApiClient
import com.galaxyas.mobilepos.scanner.ScannerScreen
import com.galaxyas.mobilepos.util.formatIDR
import com.galaxyas.mobilepos.util.formatQty
import kotlinx.coroutines.delay

/**
 * Data Sheet: cek harga & stok cepat di lantai toko — teks besar, read-only,
 * scan atau ketik. (Versi desktop juga bisa edit massal; di HP sengaja
 * read-only supaya cepat dan aman dipakai sambil berdiri.)
 */
@Composable
fun DataSheetScreen(api: ApiClient) {
    var query by remember { mutableStateOf("") }
    var results by remember { mutableStateOf<List<ProductWithStock>>(emptyList()) }
    var loading by remember { mutableStateOf(false) }
    var scanning by remember { mutableStateOf(false) }

    LaunchedEffect(query) {
        if (query.isBlank()) { results = emptyList(); return@LaunchedEffect }
        delay(300)
        loading = true
        results = runCatching { api.listProducts(query.trim(), true, 40) }.getOrDefault(emptyList())
        loading = false
    }

    if (scanning) {
        ScannerScreen(
            onResult = { query = it; scanning = false },
            onClose = { scanning = false },
        )
        return
    }

    Column(Modifier.fillMaxSize().padding(horizontal = 12.dp)) {
        OutlinedTextField(
            value = query, onValueChange = { query = it },
            label = { Text("Cari nama / barcode") }, singleLine = true,
            modifier = Modifier.fillMaxWidth().padding(vertical = 8.dp),
            trailingIcon = {
                IconButton(onClick = { scanning = true }) {
                    Icon(Icons.Default.QrCodeScanner, contentDescription = "Scan")
                }
            },
        )
        LazyColumn(verticalArrangement = Arrangement.spacedBy(8.dp)) {
            items(results, key = { it.id }) { p ->
                Card(Modifier.fillMaxWidth()) {
                    Column(Modifier.fillMaxWidth().padding(14.dp)) {
                        Text(p.name, style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold)
                        p.barcode?.let {
                            Text(it, style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant)
                        }
                        Row(Modifier.fillMaxWidth().padding(top = 8.dp), verticalAlignment = Alignment.Bottom) {
                            Column(Modifier.weight(1f)) {
                                Text("Harga", style = MaterialTheme.typography.labelSmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant)
                                Text(
                                    formatIDR(p.sell_price),
                                    style = MaterialTheme.typography.headlineSmall,
                                    fontWeight = FontWeight.ExtraBold,
                                    color = MaterialTheme.colorScheme.secondary,
                                )
                            }
                            Column(horizontalAlignment = Alignment.End) {
                                Text("Stok", style = MaterialTheme.typography.labelSmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant)
                                Text(
                                    formatQty(p.stock_qty),
                                    style = MaterialTheme.typography.headlineSmall,
                                    fontWeight = FontWeight.Bold,
                                )
                            }
                        }
                        if (!p.is_active) {
                            Text("(nonaktif)", color = MaterialTheme.colorScheme.error,
                                style = MaterialTheme.typography.bodySmall)
                        }
                    }
                }
            }
            item {
                val label = when {
                    loading -> "Mencari…"
                    query.isBlank() -> "Scan atau ketik untuk cek harga & stok."
                    results.isEmpty() -> "Tidak ditemukan."
                    else -> ""
                }
                if (label.isNotEmpty()) {
                    Text(label, color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(16.dp))
                }
            }
        }
    }
}
