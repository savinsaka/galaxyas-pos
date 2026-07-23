package com.galaxyas.mobilepos.ui.theme

import androidx.compose.material3.ColorScheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.ui.graphics.Color

/**
 * 6 tema dari desktop/src/app.css, dipetakan ke Material3 ColorScheme.
 * Konvensi pemetaan CSS var -> slot M3:
 *   --primary -> primary, --baby-blue-soft -> primaryContainer, --bg -> background,
 *   --panel -> surface, --baby-blue-bg -> surfaceVariant, --text -> onSurface,
 *   --text-dim -> onSurfaceVariant, --border-strong -> outline, --border -> outlineVariant,
 *   --danger -> error, --primary-dark -> secondary. --success/--warning lewat AppExtraColors.
 * Key setting `theme` sama seperti desktop: "" / baby-blue (default), dark, forest,
 * sunset, grape, high-contrast.
 */
data class AppTheme(
    val key: String,
    val label: String,
    val swatch: Color,
    val scheme: ColorScheme,
    val extra: AppExtraColors,
    val isDark: Boolean = false,
)

data class AppExtraColors(
    val success: Color,
    val warning: Color,
)

private val BabyBlue = AppTheme(
    key = "baby-blue",
    label = "Baby Blue (Default)",
    swatch = Color(0xFF4A90D9),
    isDark = false,
    extra = AppExtraColors(success = Color(0xFF2E9E6B), warning = Color(0xFFE0A23B)),
    scheme = lightColorScheme(
        primary = Color(0xFF4A90D9),
        onPrimary = Color.White,
        primaryContainer = Color(0xFFD6ECFB),
        onPrimaryContainer = Color(0xFF1F3A52),
        secondary = Color(0xFF2F6AA8),
        onSecondary = Color.White,
        tertiary = Color(0xFF2E9E6B),
        onTertiary = Color.White,
        background = Color(0xFFEEF6FD),
        onBackground = Color(0xFF1F3A52),
        surface = Color(0xFFFFFFFF),
        onSurface = Color(0xFF1F3A52),
        surfaceVariant = Color(0xFFF2F9FF),
        onSurfaceVariant = Color(0xFF6B8299),
        outline = Color(0xFFA9CDEC),
        outlineVariant = Color(0xFFCFE3F5),
        error = Color(0xFFD9534F),
        onError = Color.White,
    ),
)

private val Dark = AppTheme(
    key = "dark",
    label = "Dark Mode",
    swatch = Color(0xFF1B2733),
    isDark = true,
    extra = AppExtraColors(success = Color(0xFF4CBF8A), warning = Color(0xFFF0B862)),
    scheme = darkColorScheme(
        primary = Color(0xFF5AA3F0),
        onPrimary = Color(0xFF10171F),
        primaryContainer = Color(0xFF24313F),
        onPrimaryContainer = Color(0xFFE6EDF3),
        secondary = Color(0xFF4A90D9),
        onSecondary = Color(0xFF10171F),
        tertiary = Color(0xFF4CBF8A),
        onTertiary = Color(0xFF10171F),
        background = Color(0xFF10171F),
        onBackground = Color(0xFFE6EDF3),
        surface = Color(0xFF1E2A36),
        onSurface = Color(0xFFE6EDF3),
        surfaceVariant = Color(0xFF1A2530),
        onSurfaceVariant = Color(0xFF93A4B3),
        outline = Color(0xFF3D5266),
        outlineVariant = Color(0xFF2E3F4F),
        error = Color(0xFFE57470),
        onError = Color(0xFF10171F),
    ),
)

private val Forest = AppTheme(
    key = "forest",
    label = "Forest",
    swatch = Color(0xFF2F9E6B),
    isDark = false,
    extra = AppExtraColors(success = Color(0xFF2E9E6B), warning = Color(0xFFE0A23B)),
    scheme = lightColorScheme(
        primary = Color(0xFF2F9E6B),
        onPrimary = Color.White,
        primaryContainer = Color(0xFFDCF1E1),
        onPrimaryContainer = Color(0xFF1F3A2C),
        secondary = Color(0xFF1E6F47),
        onSecondary = Color.White,
        tertiary = Color(0xFF2E9E6B),
        onTertiary = Color.White,
        background = Color(0xFFEEF8F1),
        onBackground = Color(0xFF1F3A2C),
        surface = Color(0xFFFFFFFF),
        onSurface = Color(0xFF1F3A2C),
        surfaceVariant = Color(0xFFF2FAF4),
        onSurfaceVariant = Color(0xFF6B8F7A),
        outline = Color(0xFFA3D9BD),
        outlineVariant = Color(0xFFCDEAD9),
        error = Color(0xFFD9534F),
        onError = Color.White,
    ),
)

private val Sunset = AppTheme(
    key = "sunset",
    label = "Sunset",
    swatch = Color(0xFFE08A3B),
    isDark = false,
    extra = AppExtraColors(success = Color(0xFF2E9E6B), warning = Color(0xFFE0A23B)),
    scheme = lightColorScheme(
        primary = Color(0xFFE08A3B),
        onPrimary = Color.White,
        primaryContainer = Color(0xFFFBE8D2),
        onPrimaryContainer = Color(0xFF4A3420),
        secondary = Color(0xFFA86123),
        onSecondary = Color.White,
        tertiary = Color(0xFF2E9E6B),
        onTertiary = Color.White,
        background = Color(0xFFFDF3E8),
        onBackground = Color(0xFF4A3420),
        surface = Color(0xFFFFFFFF),
        onSurface = Color(0xFF4A3420),
        surfaceVariant = Color(0xFFFFF8F0),
        onSurfaceVariant = Color(0xFF8F7658),
        outline = Color(0xFFE3C197),
        outlineVariant = Color(0xFFF0DCC0),
        error = Color(0xFFD9534F),
        onError = Color.White,
    ),
)

private val Grape = AppTheme(
    key = "grape",
    label = "Grape",
    swatch = Color(0xFF8B5FBF),
    isDark = false,
    extra = AppExtraColors(success = Color(0xFF2E9E6B), warning = Color(0xFFE0A23B)),
    scheme = lightColorScheme(
        primary = Color(0xFF8B5FBF),
        onPrimary = Color.White,
        primaryContainer = Color(0xFFECDFF8),
        onPrimaryContainer = Color(0xFF362A45),
        secondary = Color(0xFF613F8C),
        onSecondary = Color.White,
        tertiary = Color(0xFF2E9E6B),
        onTertiary = Color.White,
        background = Color(0xFFF5EEFB),
        onBackground = Color(0xFF362A45),
        surface = Color(0xFFFFFFFF),
        onSurface = Color(0xFF362A45),
        surfaceVariant = Color(0xFFF8F2FC),
        onSurfaceVariant = Color(0xFF83758F),
        outline = Color(0xFFCBABE3),
        outlineVariant = Color(0xFFE3D3F2),
        error = Color(0xFFD9534F),
        onError = Color.White,
    ),
)

private val HighContrast = AppTheme(
    key = "high-contrast",
    label = "Kontras Tinggi",
    swatch = Color(0xFF000000),
    isDark = false,
    extra = AppExtraColors(success = Color(0xFF0A6E2D), warning = Color(0xFF8A5A00)),
    scheme = lightColorScheme(
        primary = Color(0xFF000000),
        onPrimary = Color.White,
        primaryContainer = Color(0xFFE0E0E0),
        onPrimaryContainer = Color(0xFF000000),
        secondary = Color(0xFF000000),
        onSecondary = Color.White,
        tertiary = Color(0xFF0A6E2D),
        onTertiary = Color.White,
        background = Color(0xFFFFFFFF),
        onBackground = Color(0xFF000000),
        surface = Color(0xFFFFFFFF),
        onSurface = Color(0xFF000000),
        surfaceVariant = Color(0xFFF0F0F0),
        onSurfaceVariant = Color(0xFF3D3D3D),
        outline = Color(0xFF000000),
        outlineVariant = Color(0xFF000000),
        error = Color(0xFFB30000),
        onError = Color.White,
    ),
)

val APP_THEMES: List<AppTheme> = listOf(BabyBlue, Dark, Forest, Sunset, Grape, HighContrast)

const val DEFAULT_THEME_KEY = "baby-blue"

fun themeFor(key: String?): AppTheme =
    APP_THEMES.firstOrNull { it.key == key } ?: BabyBlue
