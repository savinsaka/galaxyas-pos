package com.galaxyas.mobilepos.ui.theme

import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Shapes
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.unit.dp

/** Warna tambahan di luar slot M3 (success/warning), per tema. */
val LocalAppColors = staticCompositionLocalOf { themeFor(DEFAULT_THEME_KEY).extra }

// Radius 8dp menyamai --radius desktop.
private val AppShapes = Shapes(
    extraSmall = RoundedCornerShape(4.dp),
    small = RoundedCornerShape(8.dp),
    medium = RoundedCornerShape(8.dp),
    large = RoundedCornerShape(12.dp),
    extraLarge = RoundedCornerShape(16.dp),
)

@Composable
fun GalaxyasTheme(themeKey: String, content: @Composable () -> Unit) {
    val theme = themeFor(themeKey)
    CompositionLocalProvider(LocalAppColors provides theme.extra) {
        MaterialTheme(
            colorScheme = theme.scheme,
            shapes = AppShapes,
            content = content,
        )
    }
}
