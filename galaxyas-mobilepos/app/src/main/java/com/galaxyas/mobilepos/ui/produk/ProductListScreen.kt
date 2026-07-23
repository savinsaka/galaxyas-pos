package com.galaxyas.mobilepos.ui.produk

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material3.Card
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.galaxyas.mobilepos.data.model.ProductWithStock
import com.galaxyas.mobilepos.data.network.ApiClient
import com.galaxyas.mobilepos.util.formatIDR
import com.galaxyas.mobilepos.util.formatQty
import kotlinx.coroutines.FlowPreview
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.debounce
import kotlinx.coroutines.flow.drop
import kotlinx.coroutines.launch

private const val PAGE = 40L

data class ProductListState(
    val items: List<ProductWithStock> = emptyList(),
    val total: Long = 0,
    val loading: Boolean = false,
    val error: String? = null,
) {
    val exhausted: Boolean get() = items.size >= total
}

class ProductListViewModel(private val api: ApiClient) : ViewModel() {
    private val _state = MutableStateFlow(ProductListState())
    val state: StateFlow<ProductListState> = _state.asStateFlow()

    val search = MutableStateFlow("")

    @OptIn(FlowPreview::class)
    fun start() {
        load(reset = true)
        viewModelScope.launch {
            search.drop(1).debounce(300).collect { load(reset = true) }
        }
    }

    fun load(reset: Boolean) {
        val s = _state.value
        if (s.loading || (!reset && s.exhausted)) return
        _state.value = s.copy(loading = true, error = null)
        viewModelScope.launch {
            try {
                val page = api.listProductsPage(
                    search = search.value,
                    limit = PAGE,
                    offset = if (reset) 0 else s.items.size.toLong(),
                )
                _state.value = ProductListState(
                    items = if (reset) page.items else s.items + page.items,
                    total = page.total,
                )
            } catch (e: Exception) {
                _state.value = s.copy(loading = false, error = e.message)
            }
        }
    }
}

/** Daftar Barang (read-only di P1) — bukti alur RPC end-to-end. */
@Composable
fun ProductListScreen(vm: ProductListViewModel) {
    val state by vm.state.collectAsState()
    val query by vm.search.collectAsState()
    val listState = rememberLazyListState()

    LaunchedEffect(Unit) { vm.start() }

    // Infinite scroll: muat halaman berikut saat mendekati ujung daftar.
    val nearEnd by remember {
        derivedStateOf {
            val info = listState.layoutInfo
            val last = info.visibleItemsInfo.lastOrNull()?.index ?: 0
            last >= info.totalItemsCount - 8
        }
    }
    LaunchedEffect(nearEnd) {
        if (nearEnd && !state.loading && !state.exhausted) vm.load(reset = false)
    }

    Column(modifier = Modifier.fillMaxSize().padding(horizontal = 12.dp)) {
        OutlinedTextField(
            value = query,
            onValueChange = { vm.search.value = it },
            placeholder = { Text("Cari nama / barcode…") },
            singleLine = true,
            modifier = Modifier.fillMaxWidth().padding(vertical = 8.dp),
        )

        state.error?.let {
            Text(
                it,
                color = MaterialTheme.colorScheme.error,
                style = MaterialTheme.typography.bodySmall,
                modifier = Modifier.padding(bottom = 8.dp),
            )
        }

        LazyColumn(
            state = listState,
            verticalArrangement = Arrangement.spacedBy(8.dp),
            modifier = Modifier.fillMaxSize(),
        ) {
            items(state.items, key = { it.id }) { p -> ProductRow(p) }
            item {
                Text(
                    when {
                        state.loading -> "Memuat…"
                        state.items.isEmpty() -> "Tidak ada barang."
                        else -> "${state.total} barang."
                    },
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.fillMaxWidth().padding(vertical = 10.dp),
                )
            }
        }
    }
}

@Composable
private fun ProductRow(p: ProductWithStock) {
    Card(modifier = Modifier.fillMaxWidth()) {
        Row(
            modifier = Modifier.fillMaxWidth().padding(horizontal = 12.dp, vertical = 10.dp),
        ) {
            Column(modifier = Modifier.weight(1f)) {
                Text(p.name, fontWeight = FontWeight.SemiBold, maxLines = 1)
                Text(
                    listOfNotNull(p.brand, p.barcode ?: "tanpa barcode").joinToString(" · "),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 1,
                )
            }
            Column(horizontalAlignment = androidx.compose.ui.Alignment.End) {
                Text(
                    formatIDR(p.sell_price),
                    fontWeight = FontWeight.Bold,
                    color = MaterialTheme.colorScheme.secondary,
                )
                Text(
                    "stok ${formatQty(p.stock_qty)}",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}
