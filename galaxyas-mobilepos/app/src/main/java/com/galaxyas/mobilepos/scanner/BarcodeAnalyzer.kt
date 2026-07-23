package com.galaxyas.mobilepos.scanner

import android.annotation.SuppressLint
import androidx.camera.core.ImageProxy
import com.google.mlkit.vision.barcode.BarcodeScanner
import com.google.mlkit.vision.common.InputImage

/** Proses satu frame kamera lewat ML Kit, panggil onBarcode untuk hasil pertama. */
@SuppressLint("UnsafeOptInUsageError")
fun processFrame(scanner: BarcodeScanner, proxy: ImageProxy, onBarcode: (String) -> Unit) {
    val mediaImage = proxy.image
    if (mediaImage == null) {
        proxy.close()
        return
    }
    val image = InputImage.fromMediaImage(mediaImage, proxy.imageInfo.rotationDegrees)
    scanner.process(image)
        .addOnSuccessListener { barcodes ->
            barcodes.firstOrNull { !it.rawValue.isNullOrBlank() }?.rawValue?.let(onBarcode)
        }
        .addOnCompleteListener { proxy.close() }
}
