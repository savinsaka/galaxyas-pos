package com.galaxyas.mobilepos

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.LifecycleEventObserver
import com.galaxyas.mobilepos.ui.nav.AppRoot
import com.galaxyas.mobilepos.ui.theme.DEFAULT_THEME_KEY
import com.galaxyas.mobilepos.ui.theme.GalaxyasTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        val container = (application as PosApp).container

        setContent {
            val settings by container.settings.settings.collectAsState()
            val themeKey = settings["theme"].takeUnless { it.isNullOrBlank() } ?: DEFAULT_THEME_KEY

            // Cek ulang koneksi tiap app kembali ke depan (HP sering lepas wifi).
            val lifecycleOwner = LocalLifecycleOwner.current
            DisposableEffect(lifecycleOwner) {
                val observer = LifecycleEventObserver { _, event ->
                    if (event == Lifecycle.Event.ON_RESUME) container.connectionWatcher.checkAsync()
                }
                lifecycleOwner.lifecycle.addObserver(observer)
                onDispose { lifecycleOwner.lifecycle.removeObserver(observer) }
            }

            GalaxyasTheme(themeKey = themeKey) {
                AppRoot(container)
            }
        }
    }
}
