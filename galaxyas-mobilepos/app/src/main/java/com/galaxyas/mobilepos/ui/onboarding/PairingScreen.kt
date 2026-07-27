package com.galaxyas.mobilepos.ui.onboarding

import android.os.Build
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import com.galaxyas.mobilepos.data.PairingQrResult
import com.galaxyas.mobilepos.data.ServerRegistry
import com.galaxyas.mobilepos.data.network.ConnMode
import com.galaxyas.mobilepos.data.network.RemoteConfig
import com.galaxyas.mobilepos.data.network.RpcClient
import com.galaxyas.mobilepos.data.parsePairingQr
import com.galaxyas.mobilepos.scanner.ScannerScreen
import kotlinx.coroutines.launch

/** Nama yang muncul di daftar "Perangkat Terhubung" pada PC kasir. */
private fun deviceName(): String =
    listOf(Build.MANUFACTURER, Build.MODEL)
        .filter { it.isNotBlank() }
        .joinToString(" ")
        .trim()
        .ifBlank { "HP Kasir" }

/**
 * Pendaftaran HP ke Server Pusat. Kode pairing 6 karakter ditukar SEKALI dengan
 * token panjang milik HP ini (`POST /pair`), jadi kode pendek itu tidak pernah
 * dipakai lagi untuk permintaan berikutnya — penting karena Server Pusat kini
 * juga bisa dijangkau lewat internet.
 *
 * Satu entri boleh menyimpan dua alamat: LAN (wifi toko) dan relay (dari mana
 * saja). Cukup isi salah satu, tapi mengisi keduanya bikin HP bisa dipakai di
 * kedua situasi tanpa daftar ulang.
 */
@Composable
fun PairingScreen(
    registry: ServerRegistry,
    rpc: RpcClient,
    onPaired: () -> Unit,
    onCancel: (() -> Unit)? = null,
) {
    val scope = rememberCoroutineScope()

    var name by remember { mutableStateOf("") }
    var host by remember { mutableStateOf("") }
    var port by remember { mutableStateOf("8899") }
    var relayUrl by remember { mutableStateOf("") }
    var storeId by remember { mutableStateOf("") }
    var code by remember { mutableStateOf("") }
    var busy by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    var scanning by remember { mutableStateOf(false) }
    var scanInfo by remember { mutableStateOf<String?>(null) }

    /** Isi semua kolom dari QR di layar Pengaturan PC. */
    fun applyQr(raw: String) {
        when (val result = parsePairingQr(raw)) {
            is PairingQrResult.Ok -> {
                val p = result.payload
                if (name.isBlank()) name = p.name
                host = p.host
                port = p.port.toString()
                relayUrl = p.relay
                storeId = p.store_id
                code = p.code
                error = null
                scanInfo = buildString {
                    append("Terbaca: ${p.name}")
                    if (p.host.isNotBlank()) append(" · LOCAL ${p.host}:${p.port}")
                    append(if (p.relay.isNotBlank()) " · ONLINE ${p.relay}" else " · ONLINE belum aktif di PC")
                }
            }
            is PairingQrResult.TooNew -> {
                scanInfo = null
                error = "QR ini dari GALAXYAS POS versi lebih baru. Update dulu aplikasi HP-nya."
            }
            PairingQrResult.NotPairingQr -> {
                scanInfo = null
                error = "Itu bukan QR pairing. Di PC: Pengaturan → Server Pusat → Tampilkan QR."
            }
        }
    }

    if (scanning) {
        ScannerScreen(
            onResult = { raw ->
                applyQr(raw)
                scanning = false
            },
            onClose = { scanning = false },
        )
        return
    }

    fun connectNew() {
        val p = port.toIntOrNull() ?: 8899
        val hasLan = host.isNotBlank()
        val hasOnline = relayUrl.isNotBlank() && storeId.isNotBlank()
        if (name.isBlank() || code.isBlank()) {
            error = "Nama Server dan Kode Pairing wajib diisi."
            return
        }
        if (!hasLan && !hasOnline) {
            error = "Isi minimal satu jalur: IP lokal, atau URL Relay + Store ID."
            return
        }

        val draft = registry.newServer(
            name = name.trim(),
            lanHost = host.trim(),
            lanPort = p,
            relayUrl = relayUrl.trim(),
            storeId = storeId.trim(),
            deviceToken = "",
        )
        // Pairing lewat LAN bila tersedia: saat mendaftar, HP biasanya masih di
        // toko, dan jalur lokal tidak bergantung pada relay sudah jalan atau
        // belum. Token yang didapat berlaku untuk kedua jalur.
        val mode = if (hasLan) ConnMode.LOCAL else ConnMode.ONLINE
        val remote: RemoteConfig = draft.toRemote(mode) ?: run {
            error = "Alamat server tidak lengkap."
            return
        }

        busy = true
        error = null
        scope.launch {
            try {
                val result = rpc.pair(remote, code.trim().uppercase(), deviceName())
                registry.add(draft.copy(device_token = result.device_token), mode)
                onPaired()
            } catch (e: Exception) {
                error = e.message ?: "Gagal terhubung."
            } finally {
                busy = false
            }
        }
    }

    Column(
        modifier = Modifier.fillMaxSize().verticalScroll(rememberScrollState())
            .padding(24.dp).imePadding(),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        Text(
            "GALAXYAS POS",
            style = MaterialTheme.typography.headlineMedium,
            fontWeight = FontWeight.ExtraBold,
            color = MaterialTheme.colorScheme.secondary,
        )
        Text("Hubungkan ke Server Pusat", color = MaterialTheme.colorScheme.onSurfaceVariant)
        Spacer(Modifier.padding(6.dp))

        Text(
            "Aplikasi ini adalah kasir tambahan — semua data (barang, stok, transaksi) " +
                "tersimpan di PC yang menjalankan GALAXYAS POS sebagai Server Pusat.\n\n" +
                "Di PC: Pengaturan → Server Pusat. Centang \"Jadikan PC ini Server Pusat\" " +
                "untuk dapat IP dan Kode Pairing. Untuk pemakaian dari luar toko, nyalakan " +
                "juga \"Akses Online\" dan catat URL Relay serta Store ID-nya.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(Modifier.padding(6.dp))

        // Jalur utama: scan QR. Mengetik Store ID relay (32 karakter hex) di HP
        // hampir pasti salah, jadi isian manual di bawah dianggap cadangan.
        Button(
            onClick = { scanning = true },
            enabled = !busy,
            modifier = Modifier.fillMaxWidth(),
        ) { Text("📷 Scan QR dari PC") }
        Text(
            "Di PC: Pengaturan → Server Pusat → Tampilkan QR",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(top = 4.dp),
        )
        scanInfo?.let {
            Text(
                it,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.secondary,
                modifier = Modifier.fillMaxWidth().padding(top = 6.dp),
            )
        }
        Spacer(Modifier.padding(6.dp))
        HorizontalDivider()
        Text(
            "atau isi manual",
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(vertical = 6.dp),
        )

        OutlinedTextField(
            value = name, onValueChange = { name = it },
            label = { Text("Nama Server") }, placeholder = { Text("Kasir Pusat") },
            singleLine = true, enabled = !busy, modifier = Modifier.fillMaxWidth(),
        )

        Spacer(Modifier.padding(4.dp))
        HorizontalDivider()
        Text(
            "Koneksi LOCAL — saat HP di wifi toko",
            style = MaterialTheme.typography.labelLarge,
            color = MaterialTheme.colorScheme.secondary,
            modifier = Modifier.fillMaxWidth().padding(top = 8.dp),
        )
        OutlinedTextField(
            value = host, onValueChange = { host = it },
            label = { Text("IP Server Pusat") }, placeholder = { Text("192.168.1.10") },
            singleLine = true, enabled = !busy, modifier = Modifier.fillMaxWidth(),
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
        )
        OutlinedTextField(
            value = port, onValueChange = { port = it },
            label = { Text("Port") },
            singleLine = true, enabled = !busy, modifier = Modifier.fillMaxWidth(),
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
        )

        Spacer(Modifier.padding(4.dp))
        HorizontalDivider()
        Text(
            "Koneksi ONLINE — dari mana saja",
            style = MaterialTheme.typography.labelLarge,
            color = MaterialTheme.colorScheme.secondary,
            modifier = Modifier.fillMaxWidth().padding(top = 8.dp),
        )
        OutlinedTextField(
            value = relayUrl, onValueChange = { relayUrl = it },
            label = { Text("URL Relay") }, placeholder = { Text("relay.jjapps.net") },
            singleLine = true, enabled = !busy, modifier = Modifier.fillMaxWidth(),
        )
        OutlinedTextField(
            value = storeId, onValueChange = { storeId = it },
            label = { Text("Store ID") }, placeholder = { Text("lihat di PC") },
            singleLine = true, enabled = !busy, modifier = Modifier.fillMaxWidth(),
        )

        Spacer(Modifier.padding(4.dp))
        HorizontalDivider()
        OutlinedTextField(
            value = code, onValueChange = { code = it.uppercase() },
            label = { Text("Kode Pairing") }, placeholder = { Text("6 karakter, lihat di PC") },
            singleLine = true, enabled = !busy,
            modifier = Modifier.fillMaxWidth().padding(top = 8.dp),
        )
        error?.let {
            Text(
                it,
                color = MaterialTheme.colorScheme.error,
                style = MaterialTheme.typography.bodySmall,
                modifier = Modifier.padding(top = 6.dp),
            )
        }
        Spacer(Modifier.padding(4.dp))
        Row(modifier = Modifier.fillMaxWidth()) {
            Button(
                onClick = { connectNew() },
                enabled = !busy,
                modifier = Modifier.weight(1f),
            ) { Text(if (busy) "Menghubungkan…" else "Hubungkan") }
            if (onCancel != null) {
                Spacer(Modifier.width(8.dp))
                OutlinedButton(onClick = onCancel, enabled = !busy) { Text("Batal") }
            }
        }
    }
}
