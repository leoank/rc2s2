package me.leoank.rc2s2

import android.text.format.DateUtils
import androidx.compose.foundation.background
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.WrapText
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import uniffi.rc2s2_core.SessionInfo
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter

/** The last path segment, which is how a session is named everywhere. */
fun SessionInfo.name(): String = cwd.trimEnd('/').substringAfterLast('/').ifEmpty { cwd.ifEmpty { "session" } }

/** The directory above the name, with the home directory as `~`, for a second line. */
fun SessionInfo.parentPath(): String {
    val parent = cwd.trimEnd('/').substringBeforeLast('/', "")
    return parent.replace(Regex("^/(Users|home)/[^/]+"), "~")
}

fun relative(timeMs: Long): String {
    val now = System.currentTimeMillis()
    if (now - timeMs < DateUtils.MINUTE_IN_MILLIS) return "just now"
    return DateUtils.getRelativeTimeSpanString(timeMs, now, DateUtils.MINUTE_IN_MILLIS).toString()
}

private val clock: DateTimeFormatter = DateTimeFormatter.ofPattern("HH:mm")
private val dayClock: DateTimeFormatter = DateTimeFormatter.ofPattern("EEE HH:mm")

fun clockTime(timeMs: Long): String = clock.format(Instant.ofEpochMilli(timeMs).atZone(ZoneId.systemDefault()))

/** "14:05" today, "Tue 14:05" on another day. */
fun whenLabel(timeMs: Long): String {
    val t = Instant.ofEpochMilli(timeMs).atZone(ZoneId.systemDefault())
    val today = Instant.now().atZone(ZoneId.systemDefault()).toLocalDate()
    return if (t.toLocalDate() == today) clock.format(t) else dayClock.format(t)
}

fun gigabytes(bytes: Double): String = "%.1f GB".format(bytes / 1_073_741_824.0)

fun thousands(n: ULong): String = when {
    n < 1000uL -> n.toString()
    n < 1_000_000uL -> "%.1fk".format(n.toDouble() / 1000.0)
    else -> "%.2fM".format(n.toDouble() / 1e6)
}

fun dollars(usd: Double): String = if (usd < 0.01) "$%.3f".format(usd) else "$%.2f".format(usd)

/** Whether monospace text (code, diffs, a tool's output) wraps at the screen's edge or
 *  scrolls sideways: one app-wide choice, toggled from the session screen and the
 *  full-screen view, remembered across launches. */
class Wrap(val on: Boolean, val toggle: () -> Unit)

val LocalWrap = compositionLocalOf { Wrap(false) {} }

/** Sideways scrolling for text that does not wrap. */
@Composable
fun Modifier.scrollUnlessWrapped(wrap: Boolean): Modifier {
    val state = rememberScrollState()
    return if (wrap) this else horizontalScroll(state)
}

@Composable
fun WrapButton() {
    val wrap = LocalWrap.current
    IconButton(onClick = wrap.toggle) {
        Icon(
            Icons.AutoMirrored.Filled.WrapText,
            if (wrap.on) "Stop wrapping lines" else "Wrap lines",
            tint = if (wrap.on) MaterialTheme.colorScheme.primary else LocalContentColor.current,
        )
    }
}

@Composable
fun StatusDot(on: Boolean, modifier: Modifier = Modifier) {
    val color = if (on) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.outlineVariant
    Box(modifier.size(10.dp).background(color, CircleShape))
}

/** A short centred line of small text, for "no sessions" and similar. */
@Composable
fun Caption(text: String, modifier: Modifier = Modifier, color: Color = MaterialTheme.colorScheme.onSurfaceVariant) {
    Text(
        text,
        style = MaterialTheme.typography.bodySmall,
        color = color,
        textAlign = TextAlign.Center,
        modifier = modifier.fillMaxWidth().padding(vertical = 8.dp),
    )
}

@Composable
fun SectionTitle(text: String, modifier: Modifier = Modifier) {
    Text(
        text,
        style = MaterialTheme.typography.titleSmall,
        color = MaterialTheme.colorScheme.primary,
        modifier = modifier.padding(top = 16.dp, bottom = 4.dp),
    )
}

@Composable
fun Empty(title: String, detail: String, modifier: Modifier = Modifier) {
    Column(modifier.fillMaxWidth().padding(32.dp), horizontalAlignment = Alignment.CenterHorizontally) {
        Text(title, style = MaterialTheme.typography.titleMedium, textAlign = TextAlign.Center)
        Text(
            detail,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            textAlign = TextAlign.Center,
            modifier = Modifier.padding(top = 8.dp),
        )
    }
}
