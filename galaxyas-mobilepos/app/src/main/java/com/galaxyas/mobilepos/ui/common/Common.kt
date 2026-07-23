package com.galaxyas.mobilepos.ui.common

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp

/** Banner merah "tidak terhubung ke Server Pusat" + tombol Coba lagi. */
@Composable
fun ConnectionBanner(checking: Boolean, onRetry: () -> Unit) {
    androidx.compose.material3.Surface(color = MaterialTheme.colorScheme.error) {
        Row(
            modifier = Modifier.fillMaxWidth().padding(horizontal = 12.dp, vertical = 4.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                "⚠ Tidak terhubung ke Server Pusat",
                color = MaterialTheme.colorScheme.onError,
                style = MaterialTheme.typography.bodySmall,
                modifier = Modifier.weight(1f),
            )
            TextButton(
                onClick = onRetry,
                enabled = !checking,
                colors = ButtonDefaults.textButtonColors(contentColor = Color.White),
            ) {
                Text(if (checking) "Memeriksa…" else "Coba lagi")
            }
        }
    }
}

/** Placeholder layar yang belum dibangun di fase ini. */
@Composable
fun ComingSoon(icon: String, title: String, note: String) {
    Column(
        modifier = Modifier.fillMaxWidth().padding(24.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        Text(icon, style = MaterialTheme.typography.displayMedium)
        Text(title, style = MaterialTheme.typography.titleMedium)
        Text(
            note,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}
