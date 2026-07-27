package com.galaxyas.mobilepos.scanner

import android.annotation.SuppressLint
import androidx.camera.core.ImageProxy
import com.google.mlkit.vision.barcode.BarcodeScanner
import com.google.mlkit.vision.common.InputImage
import java.util.concurrent.Executor

/**
 * Proses satu frame kamera lewat ML Kit, panggil onBarcode untuk hasil pertama.
 *
 * Listener sengaja dijadwalkan di [callbackExecutor] (thread analisis), bukan
 * main thread bawaan Tasks API: proxy.close() menentukan kapan frame berikutnya
 * dikirim CameraX, jadi kalau menunggu main thread yang lagi recompose, laju
 * scan ikut turun drastis.
 */
@SuppressLint("UnsafeOptInUsageError")
fun processFrame(
    scanner: BarcodeScanner,
    proxy: ImageProxy,
    callbackExecutor: Executor,
    onBarcode: (String) -> Unit,
) {
    val mediaImage = proxy.image
    if (mediaImage == null) {
        proxy.close()
        return
    }
    val image = InputImage.fromMediaImage(mediaImage, proxy.imageInfo.rotationDegrees)
    scanner.process(image)
        .addOnSuccessListener(callbackExecutor) { barcodes ->
            barcodes.firstOrNull { !it.rawValue.isNullOrBlank() }?.rawValue?.let(onBarcode)
        }
        .addOnCompleteListener(callbackExecutor) { proxy.close() }
}
