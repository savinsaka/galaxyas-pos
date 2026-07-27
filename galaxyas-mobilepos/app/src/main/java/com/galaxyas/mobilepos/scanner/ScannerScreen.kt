package com.galaxyas.mobilepos.scanner

import android.Manifest
import android.annotation.SuppressLint
import android.content.pm.PackageManager
import android.util.Size
import android.view.MotionEvent
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.camera.core.CameraSelector
import androidx.camera.core.FocusMeteringAction
import androidx.camera.core.ImageAnalysis
import androidx.camera.core.Preview
import androidx.camera.core.resolutionselector.AspectRatioStrategy
import androidx.camera.core.resolutionselector.ResolutionSelector
import androidx.camera.core.resolutionselector.ResolutionStrategy
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.camera.view.PreviewView
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.core.content.ContextCompat
import androidx.lifecycle.compose.LocalLifecycleOwner
import com.google.mlkit.vision.barcode.BarcodeScanning
import com.google.mlkit.vision.barcode.common.Barcode
import com.google.mlkit.vision.barcode.BarcodeScannerOptions
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit

/**
 * Scanner barcode kamera penuh-layar (CameraX + ML Kit bundled). Format 1D
 * umum POS + QR. onResult dipanggil sekali per hasil unik; caller memutuskan
 * apakah menutup (mode sekali) atau lanjut (mode kontinu di kasir).
 */
@Composable
fun ScannerScreen(
    continuous: Boolean,
    onResult: (String) -> Unit,
    onClose: () -> Unit,
) {
    val context = LocalContext.current
    var hasCamera by remember {
        mutableStateOf(
            ContextCompat.checkSelfPermission(context, Manifest.permission.CAMERA) ==
                PackageManager.PERMISSION_GRANTED,
        )
    }
    val permLauncher = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) {
        hasCamera = it
        if (!it) onClose()
    }
    DisposableEffect(Unit) {
        if (!hasCamera) permLauncher.launch(Manifest.permission.CAMERA)
        onDispose { }
    }

    Box(modifier = Modifier.fillMaxSize()) {
        if (hasCamera) {
            CameraPreview(onResult = onResult)
        }
        // Overlay kontrol
        Column(
            modifier = Modifier.fillMaxSize().padding(20.dp),
            verticalArrangement = Arrangement.SpaceBetween,
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Text(
                "Arahkan kamera ke barcode",
                color = Color.White,
                style = MaterialTheme.typography.titleMedium,
                modifier = Modifier.padding(top = 8.dp),
            )
            Button(onClick = onClose, modifier = Modifier.fillMaxWidth()) {
                Text(if (continuous) "Selesai Scan" else "Batal")
            }
        }
    }
}

/**
 * Debounce hasil scan. Sengaja bukan Compose state: nilainya ditulis dari thread
 * analisis, dan state Compose di sini cuma memicu recomposition sia-sia.
 */
private class ScanDebounce(private val windowMs: Long = 2_000) {
    @Volatile private var lastValue: String? = null
    @Volatile private var lastAt: Long = 0L

    fun accept(code: String): Boolean {
        val now = System.currentTimeMillis()
        if (code == lastValue && now - lastAt <= windowMs) return false
        lastValue = code
        lastAt = now
        return true
    }
}

@SuppressLint("ClickableViewAccessibility")
@Composable
private fun CameraPreview(onResult: (String) -> Unit) {
    val lifecycleOwner = LocalLifecycleOwner.current
    val analysisExecutor = remember { Executors.newSingleThreadExecutor() }

    val scanner = remember {
        BarcodeScanning.getClient(
            BarcodeScannerOptions.Builder().setBarcodeFormats(
                Barcode.FORMAT_EAN_13, Barcode.FORMAT_EAN_8,
                Barcode.FORMAT_UPC_A, Barcode.FORMAT_UPC_E,
                Barcode.FORMAT_CODE_39, Barcode.FORMAT_CODE_128,
                Barcode.FORMAT_QR_CODE,
            ).build(),
        )
    }
    val debounce = remember { ScanDebounce() }

    DisposableEffect(Unit) {
        onDispose {
            analysisExecutor.shutdown()
            scanner.close()
        }
    }

    AndroidView(
        modifier = Modifier.fillMaxSize(),
        factory = { ctx ->
            val previewView = PreviewView(ctx).apply {
                // FIT_CENTER: yang terlihat = persis yang dianalisis. Dengan
                // FILL_CENTER frame di-crop, jadi barcode yang "kelihatan pas"
                // bisa saja di luar buffer yang masuk ke ML Kit.
                scaleType = PreviewView.ScaleType.FIT_CENTER
            }
            val providerFuture = ProcessCameraProvider.getInstance(ctx)
            providerFuture.addListener({
                val provider = providerFuture.get()
                // Satu ResolutionSelector dipakai bersama supaya preview dan
                // analisis melihat bidang yang sama. 1080p penting untuk 1D:
                // EAN-13 butuh ~95 modul, di 720p barcode kecil tinggal ~3 px
                // per modul dan gagal terbaca begitu agak miring/jauh.
                val resolution = ResolutionSelector.Builder()
                    .setAspectRatioStrategy(AspectRatioStrategy.RATIO_16_9_FALLBACK_AUTO_STRATEGY)
                    .setResolutionStrategy(
                        ResolutionStrategy(
                            Size(1920, 1080),
                            ResolutionStrategy.FALLBACK_RULE_CLOSEST_HIGHER_THEN_LOWER,
                        ),
                    )
                    .build()
                val preview = Preview.Builder()
                    .setResolutionSelector(resolution)
                    .build()
                    .also { it.surfaceProvider = previewView.surfaceProvider }
                val analysis = ImageAnalysis.Builder()
                    .setResolutionSelector(resolution)
                    .setBackpressureStrategy(ImageAnalysis.STRATEGY_KEEP_ONLY_LATEST)
                    .build()
                analysis.setAnalyzer(analysisExecutor) { proxy ->
                    processFrame(scanner, proxy, analysisExecutor) { code ->
                        if (debounce.accept(code)) previewView.post { onResult(code) }
                    }
                }
                provider.unbindAll()
                val camera = provider.bindToLifecycle(
                    lifecycleOwner, CameraSelector.DEFAULT_BACK_CAMERA, preview, analysis,
                )
                // Tap-to-focus: autofokus kontinu sering "berburu" pada barcode
                // jarak dekat, tap memaksa kunci fokus di titik yang dituju.
                previewView.setOnTouchListener { view, event ->
                    if (event.action == MotionEvent.ACTION_UP) {
                        val point = previewView.meteringPointFactory.createPoint(event.x, event.y)
                        camera.cameraControl.startFocusAndMetering(
                            FocusMeteringAction.Builder(
                                point,
                                FocusMeteringAction.FLAG_AF or FocusMeteringAction.FLAG_AE,
                            ).setAutoCancelDuration(3, TimeUnit.SECONDS).build(),
                        )
                        view.performClick()
                    }
                    true
                }
            }, ContextCompat.getMainExecutor(ctx))
            previewView
        },
    )
}
