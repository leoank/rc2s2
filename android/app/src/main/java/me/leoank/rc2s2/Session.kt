package me.leoank.rc2s2

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.DragInteraction
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.Send
import androidx.compose.material.icons.filled.Build
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material.icons.filled.Language
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Stop
import androidx.compose.material.icons.filled.Terminal
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilledIconButton
import androidx.compose.material3.FilledTonalIconButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SmallFloatingActionButton
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import uniffi.rc2s2_core.Item
import uniffi.rc2s2_core.Role
import uniffi.rc2s2_core.SessionInfo

/** One session: its requests waiting at the top, the conversation live, and a prompt
 *  box; interrupt and kill when the session runs under `rc2s2 run`. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SessionScreen(
    model: HubModel,
    snackbar: SnackbarHostState,
    nodeId: String,
    sessionId: String,
    back: () -> Unit,
    openHealth: () -> Unit,
) {
    val live = model.sessions.firstOrNull { it.nodeId == nodeId && it.id == sessionId }
    val s = live ?: model.openSession ?: return
    val ended = live == null
    val node = model.node(nodeId)?.name ?: "node"
    var menu by remember { mutableStateOf(false) }
    var confirmKill by remember { mutableStateOf(false) }
    var text by remember(sessionId) { mutableStateOf("") }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Column {
                        Text(s.name(), maxLines = 1, overflow = TextOverflow.Ellipsis)
                        Text(
                            listOf(
                                node,
                                when {
                                    ended -> "ended"
                                    model.pendingFor(s).isNotEmpty() -> "waiting for you"
                                    s.busy -> "working"
                                    else -> "idle"
                                },
                                if (s.wrapped) "rc2s2 run" else "started outside rc2s2",
                            ).joinToString(" · "),
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                        )
                    }
                },
                navigationIcon = { IconButton(onClick = back) { Icon(Icons.AutoMirrored.Filled.ArrowBack, "Back") } },
                actions = {
                    Box {
                        IconButton(onClick = { menu = true }) { Icon(Icons.Filled.MoreVert, "More") }
                        DropdownMenu(expanded = menu, onDismissRequest = { menu = false }) {
                            DropdownMenuItem(text = { Text("Node health") }, onClick = { menu = false; openHealth() })
                            if (s.wrapped && !ended) {
                                DropdownMenuItem(
                                    text = { Text("Interrupt") },
                                    onClick = { menu = false; model.interrupt(s, null) },
                                )
                                DropdownMenuItem(
                                    text = { Text("Kill session", color = MaterialTheme.colorScheme.error) },
                                    onClick = { menu = false; confirmKill = true },
                                )
                            }
                        }
                    }
                },
            )
        },
        snackbarHost = { SnackbarHost(snackbar) },
        bottomBar = {
            if (!ended) {
                PromptBar(
                    text, { text = it },
                    wrapped = s.wrapped,
                    busy = s.busy,
                    send = { model.prompt(s, text.trim()); text = "" },
                    stop = { model.interrupt(s, text.trim()); text = "" },
                )
            }
        },
    ) { padding ->
        Column(Modifier.fillMaxSize().padding(padding)) {
            if (ended) {
                Surface(color = MaterialTheme.colorScheme.surfaceVariant, modifier = Modifier.fillMaxWidth()) {
                    Caption("This session has ended")
                }
            }
            Conversation(model, s, Modifier.weight(1f))
        }
    }

    if (confirmKill) {
        AlertDialog(
            onDismissRequest = { confirmKill = false },
            title = { Text("Kill this session?") },
            text = { Text("Claude Code in ${s.cwd} is stopped at once. Nothing is saved beyond its transcript.") },
            confirmButton = {
                TextButton(onClick = { confirmKill = false; model.kill(s); back() }) {
                    Text("Kill", color = MaterialTheme.colorScheme.error)
                }
            },
            dismissButton = { TextButton(onClick = { confirmKill = false }) { Text("Cancel") } },
        )
    }
}

@Composable
private fun Conversation(model: HubModel, s: SessionInfo, modifier: Modifier) {
    val list = rememberLazyListState()
    val scope = rememberCoroutineScope()
    val items = model.conversation
    val waiting = model.pendingFor(s)
    // Follow new items while the person stays at the end. Only their own drag decides
    // otherwise (a programmatic scroll never does): once it settles, follow again if it
    // ended at the bottom, else stop until they come back down or tap the jump button.
    val atBottom by remember { derivedStateOf { !list.canScrollForward } }
    var stick by remember { mutableStateOf(true) }
    var dragged by remember { mutableStateOf(false) }
    LaunchedEffect(list) {
        list.interactionSource.interactions.collect { if (it is DragInteraction.Start) dragged = true }
    }
    LaunchedEffect(list.isScrollInProgress) {
        if (!list.isScrollInProgress && dragged) {
            dragged = false
            stick = !list.canScrollForward
        }
    }
    val count = items.size + waiting.size
    LaunchedEffect(count) {
        if (count > 0 && stick) list.scrollToItem(count - 1, Int.MAX_VALUE)
    }
    Box(modifier.fillMaxSize()) {
        if (items.isEmpty() && waiting.isEmpty()) {
            Empty("Nothing here yet", "The conversation appears as the session's transcript is written.")
        }
        LazyColumn(
            state = list,
            contentPadding = PaddingValues(horizontal = 16.dp, vertical = 12.dp),
            verticalArrangement = Arrangement.spacedBy(6.dp),
            modifier = Modifier.fillMaxSize(),
        ) {
            itemsIndexed(items, key = { i, it -> "${it.seq}-$i" }) { i, item ->
                val before = items.getOrNull(i - 1)
                if (before == null || item.timeMs - before.timeMs > 10 * 60_000) {
                    Caption(whenLabel(item.timeMs), Modifier.padding(top = if (before == null) 0.dp else 8.dp))
                }
                ConversationItem(item)
            }
            itemsIndexed(waiting, key = { _, p -> "p" + p.id }) { _, p ->
                PendingCard(model, p, null, Modifier.padding(top = 8.dp))
            }
        }
        AnimatedVisibility(
            visible = !atBottom && items.isNotEmpty(),
            modifier = Modifier.align(Alignment.BottomEnd).padding(16.dp),
        ) {
            SmallFloatingActionButton(onClick = {
                stick = true
                scope.launch { list.animateScrollToItem(count - 1, Int.MAX_VALUE) }
            }) {
                Icon(Icons.Filled.KeyboardArrowDown, "Jump to latest")
            }
        }
    }
}

@Composable
private fun ConversationItem(item: Item) {
    when (item.role) {
        Role.USER -> BoxWithConstraints(Modifier.fillMaxWidth()) {
            Surface(
                color = MaterialTheme.colorScheme.primaryContainer,
                shape = RoundedCornerShape(16.dp, 4.dp, 16.dp, 16.dp),
                modifier = Modifier.align(Alignment.CenterEnd).widthIn(max = maxWidth * 0.85f),
            ) {
                Text(item.text, style = MaterialTheme.typography.bodyLarge, modifier = Modifier.padding(12.dp))
            }
        }
        Role.ASSISTANT -> Folded(item.text)
        Role.TOOL -> ToolRow(item.text)
        Role.TOOL_RESULT -> ToolResult(item.text)
        Role.SYSTEM -> Caption(item.text)
        Role.DAEMON -> Caption(item.text.uppercase(), color = MaterialTheme.colorScheme.error)
    }
}

/** A reply longer than a screen or two is folded until tapped. */
@Composable
private fun Folded(text: String) {
    var open by remember(text) { mutableStateOf(false) }
    val long = text.count { it == '\n' } > 30 || text.length > 2000
    Column(Modifier.fillMaxWidth().padding(vertical = 4.dp).clickable(enabled = long) { open = !open }) {
        Text(
            text,
            style = MaterialTheme.typography.bodyLarge,
            maxLines = if (open || !long) Int.MAX_VALUE else 30,
            overflow = TextOverflow.Ellipsis,
        )
        if (long) {
            Text(
                if (open) "Show less" else "Show all",
                style = MaterialTheme.typography.labelLarge,
                color = MaterialTheme.colorScheme.primary,
                modifier = Modifier.padding(top = 4.dp),
            )
        }
    }
}

private fun toolIcon(text: String): ImageVector = when (text.substringBefore(':').trim()) {
    "Bash" -> Icons.Filled.Terminal
    "Edit", "Write", "NotebookEdit" -> Icons.Filled.Edit
    "Read", "Grep", "Glob", "Search" -> Icons.Filled.Search
    "WebFetch", "WebSearch" -> Icons.Filled.Language
    else -> Icons.Filled.Build
}

@Composable
private fun ToolRow(text: String) {
    val name = text.substringBefore(':')
    val what = if (':' in text) text.substringAfter(':').trim() else ""
    Row(
        verticalAlignment = Alignment.Top,
        modifier = Modifier
            .fillMaxWidth()
            .background(MaterialTheme.colorScheme.surfaceContainerHigh, RoundedCornerShape(10.dp))
            .padding(horizontal = 10.dp, vertical = 8.dp),
    ) {
        Icon(toolIcon(text), null, tint = MaterialTheme.colorScheme.secondary, modifier = Modifier.size(18.dp))
        Spacer(Modifier.width(8.dp))
        Column {
            Text(name, style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.secondary)
            if (what.isNotEmpty()) {
                Text(
                    what,
                    fontFamily = FontFamily.Monospace,
                    style = MaterialTheme.typography.bodySmall,
                    maxLines = 6,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }
    }
}

/** A result is folded to three lines until tapped; the core already clips long ones. */
@Composable
private fun ToolResult(text: String) {
    var open by remember(text) { mutableStateOf(false) }
    Text(
        text.trim().ifEmpty { "(no output)" },
        fontFamily = FontFamily.Monospace,
        style = MaterialTheme.typography.bodySmall,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        maxLines = if (open) Int.MAX_VALUE else 3,
        overflow = TextOverflow.Ellipsis,
        modifier = Modifier
            .fillMaxWidth()
            .padding(start = 28.dp)
            .clickable { open = !open },
    )
}

@Composable
private fun PromptBar(
    text: String,
    onText: (String) -> Unit,
    wrapped: Boolean,
    busy: Boolean,
    send: () -> Unit,
    stop: () -> Unit,
) {
    Surface(tonalElevation = 2.dp) {
        Column(Modifier.navigationBarsPadding().imePadding().padding(horizontal = 12.dp, vertical = 8.dp)) {
            Row(verticalAlignment = Alignment.Bottom) {
                OutlinedTextField(
                    text, onText,
                    placeholder = { Text(if (busy) "Queue a prompt for after this turn" else "Prompt") },
                    maxLines = 5,
                    modifier = Modifier.weight(1f),
                )
                Spacer(Modifier.width(8.dp))
                if (wrapped) {
                    FilledTonalIconButton(onClick = stop) {
                        Icon(Icons.Filled.Stop, if (text.isBlank()) "Interrupt" else "Interrupt, then send")
                    }
                    Spacer(Modifier.width(4.dp))
                }
                FilledIconButton(onClick = send, enabled = text.isNotBlank()) {
                    Icon(Icons.AutoMirrored.Filled.Send, "Send")
                }
            }
            if (wrapped && text.isNotBlank()) {
                Text(
                    "Send queues it behind the current turn; the stop button interrupts and sends it instead.",
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(top = 4.dp, start = 4.dp),
                )
            }
        }
    }
}
