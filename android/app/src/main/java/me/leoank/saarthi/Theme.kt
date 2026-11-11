package me.leoank.saarthi

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color

/** Warm ivory and terracotta in light, warm charcoal in dark, after the Claude apps'
 *  palette; the same on every phone rather than the wallpaper's colours. */
private val Light = lightColorScheme(
    primary = Color(0xFFC96442),
    onPrimary = Color(0xFFFFFFFF),
    primaryContainer = Color(0xFFF2E3DA),
    onPrimaryContainer = Color(0xFF5A2614),
    secondary = Color(0xFF8A5A44),
    onSecondary = Color(0xFFFFFFFF),
    secondaryContainer = Color(0xFFEDE6DA),
    onSecondaryContainer = Color(0xFF3D3929),
    tertiary = Color(0xFFB0661E),
    onTertiary = Color(0xFFFFFFFF),
    tertiaryContainer = Color(0xFFF8E3C8),
    onTertiaryContainer = Color(0xFF4A2A06),
    background = Color(0xFFFAF9F5),
    onBackground = Color(0xFF141413),
    surface = Color(0xFFFAF9F5),
    onSurface = Color(0xFF141413),
    surfaceVariant = Color(0xFFF0EEE6),
    onSurfaceVariant = Color(0xFF6B6A63),
    surfaceContainerLowest = Color(0xFFFFFFFF),
    surfaceContainerLow = Color(0xFFF5F4EE),
    surfaceContainer = Color(0xFFF0EEE6),
    surfaceContainerHigh = Color(0xFFEAE8DF),
    surfaceContainerHighest = Color(0xFFE3E1D7),
    outline = Color(0xFFB9B6AB),
    outlineVariant = Color(0xFFDEDCD1),
    error = Color(0xFFB3261E),
    onError = Color(0xFFFFFFFF),
    errorContainer = Color(0xFFF9DEDC),
    onErrorContainer = Color(0xFF410E0B),
)

private val Dark = darkColorScheme(
    primary = Color(0xFFD97757),
    onPrimary = Color(0xFF1F1E1D),
    primaryContainer = Color(0xFF4A3026),
    onPrimaryContainer = Color(0xFFF6D8CB),
    secondary = Color(0xFFD9A58C),
    onSecondary = Color(0xFF2B1A12),
    secondaryContainer = Color(0xFF3A3834),
    onSecondaryContainer = Color(0xFFEDE9DE),
    tertiary = Color(0xFFE5A85C),
    onTertiary = Color(0xFF2E1A02),
    tertiaryContainer = Color(0xFF4D3514),
    onTertiaryContainer = Color(0xFFF8E3C8),
    background = Color(0xFF262624),
    onBackground = Color(0xFFF5F4EE),
    surface = Color(0xFF262624),
    onSurface = Color(0xFFF5F4EE),
    surfaceVariant = Color(0xFF30302E),
    onSurfaceVariant = Color(0xFFA9A69D),
    surfaceContainerLowest = Color(0xFF1F1E1D),
    surfaceContainerLow = Color(0xFF2A2A27),
    surfaceContainer = Color(0xFF30302E),
    surfaceContainerHigh = Color(0xFF363633),
    surfaceContainerHighest = Color(0xFF3D3D3A),
    outline = Color(0xFF6E6C66),
    outlineVariant = Color(0xFF45443F),
    error = Color(0xFFF2B8B5),
    onError = Color(0xFF601410),
    errorContainer = Color(0xFF8C1D18),
    onErrorContainer = Color(0xFFF9DEDC),
)

@Composable
fun SaarthiTheme(content: @Composable () -> Unit) {
    MaterialTheme(colorScheme = if (isSystemInDarkTheme()) Dark else Light, content = content)
}
