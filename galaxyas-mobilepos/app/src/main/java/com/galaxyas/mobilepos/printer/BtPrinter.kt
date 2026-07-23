package com.galaxyas.mobilepos.printer

import android.Manifest
import android.annotation.SuppressLint
import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothManager
import android.bluetooth.BluetoothSocket
import android.content.Context
import android.content.pm.PackageManager
import android.os.Build
import androidx.core.content.ContextCompat
import java.io.IOException
import java.util.UUID
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeout

/** Printer thermal yang sudah dipasangkan (bonded) di Settings Android. */
data class BondedPrinter(val name: String, val mac: String)

/**
 * Cetak byte ESC/POS ke printer thermal via Bluetooth Classic SPP (RFCOMM).
 * Tanpa discovery — user memasangkan printer sekali lewat Settings Android,
 * jadi tidak perlu permission lokasi; hanya BLUETOOTH_CONNECT (API 31+).
 * Connect-per-print: sederhana & self-healing (socket basi tidak menumpuk).
 */
object BtPrinter {
    private val SPP_UUID: UUID = UUID.fromString("00001101-0000-1000-8000-00805F9B34FB")
    private const val CHUNK = 512
    private const val CHUNK_DELAY_MS = 20L

    /** Permission yang perlu diminta untuk API level saat ini. */
    fun requiredPermission(): String? =
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) Manifest.permission.BLUETOOTH_CONNECT else null

    fun hasPermission(context: Context): Boolean {
        val perm = requiredPermission() ?: return true
        return ContextCompat.checkSelfPermission(context, perm) == PackageManager.PERMISSION_GRANTED
    }

    private fun adapter(context: Context): BluetoothAdapter? =
        (context.getSystemService(Context.BLUETOOTH_SERVICE) as? BluetoothManager)?.adapter

    @SuppressLint("MissingPermission") // dipagari hasPermission() oleh pemanggil
    fun bondedPrinters(context: Context): List<BondedPrinter> {
        if (!hasPermission(context)) return emptyList()
        val ad = adapter(context) ?: return emptyList()
        return ad.bondedDevices.orEmpty().map { BondedPrinter(it.name ?: it.address, it.address) }
    }

    class BtPrintException(message: String) : Exception(message)

    /**
     * Kirim byte ke printer. Kembalikan Result agar pemanggil bisa menampilkan
     * pesan ramah tanpa try/catch. Semua kerja di Dispatchers.IO, timeout 10 dtk.
     */
    @SuppressLint("MissingPermission")
    suspend fun print(context: Context, mac: String, bytes: ByteArray): Result<Unit> =
        withContext(Dispatchers.IO) {
            runCatching {
                if (!hasPermission(context)) {
                    throw BtPrintException("Izin Bluetooth belum diberikan.")
                }
                val ad = adapter(context) ?: throw BtPrintException("Perangkat tidak mendukung Bluetooth.")
                if (!ad.isEnabled) throw BtPrintException("Bluetooth mati — nyalakan dulu di pengaturan HP.")
                val device = try {
                    ad.getRemoteDevice(mac)
                } catch (_: IllegalArgumentException) {
                    throw BtPrintException("Alamat printer tidak valid.")
                }
                ad.cancelDiscovery()

                withTimeout(10_000) {
                    var socket: BluetoothSocket? = null
                    try {
                        socket = try {
                            device.createRfcommSocketToServiceRecord(SPP_UUID).also { it.connect() }
                        } catch (_: IOException) {
                            // Banyak printer murah menolak socket secure — coba insecure.
                            device.createInsecureRfcommSocketToServiceRecord(SPP_UUID).also { it.connect() }
                        }
                        val out = socket.outputStream
                        var offset = 0
                        while (offset < bytes.size) {
                            val end = minOf(offset + CHUNK, bytes.size)
                            out.write(bytes, offset, end - offset)
                            out.flush()
                            offset = end
                            if (offset < bytes.size) delay(CHUNK_DELAY_MS)
                        }
                        delay(120) // beri waktu buffer printer sebelum socket ditutup
                    } catch (e: IOException) {
                        throw BtPrintException(
                            "Tidak bisa terhubung ke printer — pastikan printer menyala dan sudah dipasangkan.",
                        )
                    } finally {
                        runCatching { socket?.close() }
                    }
                }
            }
        }
}
