package me.leoank.rc2s2

import android.os.Build
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext

/** Material 3 with the phone's own colours on Android 12 and later, a teal scheme before. */
@Composable
fun Rc2s2Theme(content: @Composable () -> Unit) {
    val dark = isSystemInDarkTheme()
    val scheme = when {
        Build.VERSION.SDK_INT >= 31 -> {
            val context = LocalContext.current
            if (dark) dynamicDarkColorScheme(context) else dynamicLightColorScheme(context)
        }
        dark -> darkColorScheme(primary = Color(0xFF80CBC4), secondary = Color(0xFFB2DFDB), tertiary = Color(0xFFFFCC80))
        else -> lightColorScheme(primary = Color(0xFF00695C), secondary = Color(0xFF4DB6AC), tertiary = Color(0xFFEF6C00))
    }
    MaterialTheme(colorScheme = scheme, content = content)
}
