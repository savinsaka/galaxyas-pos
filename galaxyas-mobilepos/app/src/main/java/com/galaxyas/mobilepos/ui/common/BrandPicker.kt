package com.galaxyas.mobilepos.ui.common

import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.MenuAnchorType
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier

/**
 * Pemilih merek: satu field yang bisa diketik untuk menyaring, dengan daftar
 * merek muncul di bawahnya. Menggantikan deretan chip yang harus digeser —
 * toko dengan puluhan merek tidak praktis dicari sambil menggeser layar.
 *
 * Mengetik hanya MENYARING daftar; merek baru tidak bisa dibuat dari sini,
 * jadi teks yang tidak jadi dipilih dikembalikan ke merek terpilih saat menu
 * ditutup — supaya field tidak pernah menampilkan merek yang sebenarnya tidak
 * aktif.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun BrandPicker(
    brands: List<String>,
    selected: String?,
    onSelect: (String?) -> Unit,
    modifier: Modifier = Modifier,
    label: String = "Merek",
    /** Tampilkan pilihan "kosong" di puncak daftar (mis. mode Cari/Scan). */
    noneLabel: String? = null,
) {
    var expanded by remember { mutableStateOf(false) }
    var query by remember(selected) { mutableStateOf(selected ?: "") }

    val hits = if (query.isBlank() || query == selected) brands
    else brands.filter { it.contains(query.trim(), ignoreCase = true) }

    ExposedDropdownMenuBox(
        expanded = expanded,
        onExpandedChange = { expanded = it },
        modifier = modifier,
    ) {
        OutlinedTextField(
            value = query,
            onValueChange = { query = it; expanded = true },
            label = { Text(label) },
            placeholder = { Text(if (noneLabel != null) noneLabel else "ketik untuk mencari…") },
            singleLine = true,
            trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded) },
            modifier = Modifier
                .menuAnchor(MenuAnchorType.PrimaryEditable)
                .fillMaxWidth(),
        )
        ExposedDropdownMenu(
            expanded = expanded,
            onDismissRequest = { expanded = false; query = selected ?: "" },
        ) {
            if (noneLabel != null) {
                DropdownMenuItem(
                    text = { Text(noneLabel) },
                    onClick = { onSelect(null); query = ""; expanded = false },
                )
            }
            hits.forEach { b ->
                DropdownMenuItem(
                    text = { Text(b) },
                    onClick = { onSelect(b); query = b; expanded = false },
                )
            }
            if (hits.isEmpty()) {
                DropdownMenuItem(
                    text = { Text("Tidak ada merek cocok \"${query.trim()}\"") },
                    onClick = {},
                    enabled = false,
                )
            }
        }
    }
}
