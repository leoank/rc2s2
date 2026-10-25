package me.leoank.rc2s2

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.background
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.clickable
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.interaction.DragInteraction
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.Send
import androidx.compose.material.icons.filled.Build
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material.icons.filled.ExpandLess
import androidx.compose.material.icons.filled.ExpandMore
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
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
    var showUsage by remember { mutableStateOf(false) }
    var text by remember(sessionId) { mutableStateOf("") }
    val tokensIn = model.usage.sumOf { it.inputTokens }
    val tokensOut = model.usage.sumOf { it.outputTokens }
    val cost = model.usage.sumOf { it.costUsd }

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
                                if (tokensIn + tokensOut > 0uL) "${thousands(tokensIn)} in · ${thousands(tokensOut)} out" + (if (cost > 0.0) " · ${dollars(cost)}" else "") else null,
                            ).filterNotNull().joinToString(" · "),
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
                            DropdownMenuItem(text = { Text("Usage") }, onClick = { menu = false; showUsage = true })
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
                    waiting = model.pendingFor(s).isNotEmpty(),
                    send = { model.prompt(s, text.trim()); text = "" },
                    stop = { model.interrupt(s, text.trim()); text = "" },
                )
            }
        },
    ) { padding ->
        Column(Modifier.fillMaxSize().padding(padding)) {
            if (model.link == Link.Reconnecting) {
                Surface(color = MaterialTheme.colorScheme.errorContainer, modifier = Modifier.fillMaxWidth()) {
                    Caption("Reconnecting to ${model.hubName}…", color = MaterialTheme.colorScheme.onErrorContainer)
                }
            }
            if (ended) {
                Surface(color = MaterialTheme.colorScheme.surfaceVariant, modifier = Modifier.fillMaxWidth()) {
                    Caption("This session has ended")
                }
            }
            Conversation(model, s, Modifier.weight(1f))
        }
    }

    if (showUsage) {
        AlertDialog(
            onDismissRequest = { showUsage = false },
            title = { Text("Usage") },
            text = {
                Column {
                    if (model.usage.isEmpty()) Text("No usage counted yet.")
                    for (u in model.usage) {
                        Text(u.model, style = MaterialTheme.typography.titleSmall, modifier = Modifier.padding(top = 8.dp))
                        Text(
                            "${thousands(u.inputTokens)} in · ${thousands(u.outputTokens)} out · " +
                                "${thousands(u.cacheReadTokens)} cache read · ${thousands(u.cacheWriteTokens)} cache write\n" +
                                "${u.messages} messages" + (if (u.costUsd > 0.0) " · ${dollars(u.costUsd)} estimated" else ""),
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    Text(
                        "Counted once per message by the daemon, subagents included; the cost is an estimate from list prices.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(top = 12.dp),
                    )
                }
            },
            confirmButton = { TextButton(onClick = { showUsage = false }) { Text("Close") } },
        )
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

/** One row of the conversation: an item, with a tool call's result attached to it. */
private data class Entry(val item: Item, val result: Item?)

private fun entries(items: List<Item>): List<Entry> {
    val out = ArrayList<Entry>(items.size)
    var i = 0
    while (i < items.size) {
        val it = items[i]
        if (it.role == Role.TOOL) {
            // The result follows its call, at most a notice or two in between.
            var j = i + 1
            while (j < items.size && j <= i + 3 && items[j].role == Role.SYSTEM) j++
            if (j < items.size && items[j].role == Role.TOOL_RESULT) {
                out += Entry(it, items[j])
                for (k in i + 1 until j) out += Entry(items[k], null)
                i = j + 1
                continue
            }
        }
        out += Entry(it, null); i++
    }
    return out
}

@OptIn(ExperimentalFoundationApi::class)
@Composable
private fun Conversation(model: HubModel, s: SessionInfo, modifier: Modifier) {
    val list = rememberLazyListState()
    val scope = rememberCoroutineScope()
    val items = model.conversation
    val all = remember(items.size) { entries(items) }
    // The last screenfuls by default; the earlier rows on demand.
    var shown by remember(s.id) { mutableStateOf(60) }
    val hidden = (all.size - shown).coerceAtLeast(0)
    val rows = if (hidden > 0) all.subList(hidden, all.size) else all
    val sent = model.sending
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
    val count = (if (hidden > 0) 1 else 0) + rows.size + sent.size + waiting.size
    LaunchedEffect(count, hidden > 0) {
        if (count > 0 && stick) list.scrollToItem(count - 1, Int.MAX_VALUE)
    }
    Box(modifier.fillMaxSize()) {
        if (rows.isEmpty() && waiting.isEmpty()) {
            Empty("Nothing here yet", "The conversation appears as the session's transcript is written.")
        }
        LazyColumn(
            state = list,
            contentPadding = PaddingValues(horizontal = 16.dp, vertical = 12.dp),
            verticalArrangement = Arrangement.spacedBy(4.dp),
            modifier = Modifier.fillMaxSize(),
        ) {
            if (hidden > 0) {
                // Stays at the top while the list scrolls, so the history is never a surprise.
                stickyHeader(key = "earlier") {
                    Surface(color = MaterialTheme.colorScheme.surfaceContainerHigh, modifier = Modifier.fillMaxWidth()) {
                        TextButton(onClick = { shown += 100 }, modifier = Modifier.fillMaxWidth()) {
                            Text("$hidden earlier · show more")
                        }
                    }
                }
            }
            itemsIndexed(rows, key = { i, e -> "${e.item.seq}-$i" }) { i, e ->
                val before = rows.getOrNull(i - 1)?.item
                if (before == null || e.item.timeMs - before.timeMs > 10 * 60_000) {
                    Caption(whenLabel(e.item.timeMs), Modifier.padding(top = if (before == null) 0.dp else 8.dp))
                }
                ConversationRow(e, s.cwd, delivered = e.item.role == Role.USER && e.item.text in model.delivered)
            }
            itemsIndexed(sent, key = { _, x -> "sent" + x.timeMs }) { _, x ->
                SentTurn(x, retry = { model.retry(x) })
            }
            itemsIndexed(waiting, key = { _, p -> "p" + p.id }) { _, p ->
                PendingCard(model, p, null, Modifier.padding(top = 8.dp))
            }
        }
        AnimatedVisibility(
            visible = !atBottom && rows.isNotEmpty(),
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

/** The terminal's shapes, loosely: the person's prompt as a boxed line, Claude's text as
 *  rendered markdown, a tool call as one line with its output folded beneath, thinking
 *  folded, notices as captions. */
@Composable
private fun ConversationRow(e: Entry, cwd: String, delivered: Boolean = false) {
    val item = e.item
    when (item.role) {
        Role.USER -> UserTurn(item.text, ticks = if (delivered) 2 else 0)
        Role.ASSISTANT -> Markdown(item.text, Modifier.fillMaxWidth().padding(vertical = 4.dp))
        Role.THINKING -> Folded(
            label = "Thinking",
            summary = null,
            tint = MaterialTheme.colorScheme.onSurfaceVariant,
        ) {
            Text(
                item.text,
                style = MaterialTheme.typography.bodyMedium,
                fontStyle = FontStyle.Italic,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(start = 24.dp, top = 4.dp),
            )
        }
        Role.TOOL -> ToolCall(item, e.result, cwd)
        Role.TOOL_RESULT -> {
            var open by remember(item.seq) { mutableStateOf(false) }
            Box(Modifier.fillMaxWidth().clickable { open = !open }) { Output(item, open) }
        }
        Role.SYSTEM -> Caption(item.text)
        Role.DAEMON -> Caption(item.text.uppercase(), color = MaterialTheme.colorScheme.error)
        Role.TURN -> Text(
            item.text,
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            textAlign = TextAlign.End,
            modifier = Modifier.fillMaxWidth().padding(top = 2.dp, bottom = 6.dp),
        )
    }
}

/** A prompt sent from here, before the transcript echoes it: one tick once the hub took it,
 *  the node's note or the failure under it. */
@Composable
private fun SentTurn(x: Sent, retry: () -> Unit) {
    Column(Modifier.fillMaxWidth().clickable(enabled = x.failed != null, onClick = retry)) {
        UserTurn(x.text, ticks = if (x.accepted) 1 else 0, pending = !x.accepted && x.failed == null)
        (x.failed?.let { "$it · tap to retry" } ?: x.note)?.let {
            Text(
                it,
                style = MaterialTheme.typography.labelSmall,
                color = if (x.failed != null) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurfaceVariant,
                textAlign = TextAlign.End,
                modifier = Modifier.fillMaxWidth().padding(end = 8.dp, bottom = 4.dp),
            )
        }
    }
}

@Composable
private fun UserTurn(text: String, ticks: Int = 0, pending: Boolean = false) {
    Row(
        Modifier
            .fillMaxWidth()
            .padding(vertical = 4.dp)
            .height(IntrinsicSize.Min)
            .background(MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.35f), RoundedCornerShape(10.dp)),
    ) {
        Box(
            Modifier
                .width(4.dp)
                .fillMaxHeight()
                .background(MaterialTheme.colorScheme.primary, RoundedCornerShape(topStart = 8.dp, bottomStart = 8.dp)),
        )
        Text(
            text,
            style = MaterialTheme.typography.bodyLarge,
            fontFamily = if (text.startsWith("! ")) FontFamily.Monospace else FontFamily.Default,
            color = if (pending) MaterialTheme.colorScheme.onSurfaceVariant else MaterialTheme.colorScheme.onSurface,
            modifier = Modifier.weight(1f).padding(horizontal = 12.dp, vertical = 10.dp),
        )
        if (ticks > 0 || pending) {
            Text(
                when {
                    pending -> "○"
                    ticks >= 2 -> "✓✓"
                    else -> "✓"
                },
                style = MaterialTheme.typography.labelSmall,
                color = if (ticks >= 2) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(end = 10.dp, top = 12.dp),
            )
        }
    }
}

private fun toolIcon(tool: String): ImageVector = when (tool) {
    "Bash", "Monitor" -> Icons.Filled.Terminal
    "Edit", "Write", "NotebookEdit", "MultiEdit" -> Icons.Filled.Edit
    "Read", "Grep", "Glob", "ToolSearch" -> Icons.Filled.Search
    "WebFetch", "WebSearch" -> Icons.Filled.Language
    else -> Icons.Filled.Build
}

/** "Bash  cargo test" with its output folded to one line: the count of lines, or the
 *  error's first line; a tap opens the output and the call's detail. */
private val editTools = setOf("Edit", "Write", "NotebookEdit", "MultiEdit")

@Composable
private fun ToolCall(call: Item, result: Item?, cwd: String) {
    // An edit shows its diff at once, and so does a command of several lines (the file
    // being written, usually); everything else folds until tapped.
    val script = call.tool == "Bash" && call.detail?.contains('\n') == true
    var open by remember(call.seq) { mutableStateOf(call.tool in editTools || script) }
    val more = result != null || call.detail != null
    // A path under the session's directory reads better without it.
    val headline = call.text.removePrefix(cwd.trimEnd('/') + "/")
    Column(
        Modifier
            .fillMaxWidth()
            .clickable(enabled = more) { open = !open }
            .padding(vertical = 2.dp),
    ) {
        Row(verticalAlignment = Alignment.Top) {
            Icon(
                toolIcon(call.tool), null,
                tint = MaterialTheme.colorScheme.secondary,
                modifier = Modifier.padding(top = 2.dp).size(16.dp),
            )
            Spacer(Modifier.width(8.dp))
            Text(call.tool, style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.secondary)
            Spacer(Modifier.width(8.dp))
            Text(
                headline,
                fontFamily = FontFamily.Monospace,
                style = MaterialTheme.typography.bodySmall,
                maxLines = if (open) Int.MAX_VALUE else 2,
                overflow = TextOverflow.Ellipsis,
                modifier = Modifier.weight(1f).padding(top = 2.dp),
            )
            if (more) {
                Icon(
                    if (open) Icons.Filled.ExpandLess else Icons.Filled.ExpandMore, null,
                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.size(18.dp),
                )
            }
        }
        if (open) call.detail?.let {
            when {
                call.tool in editTools -> DiffBlock(it, Modifier.padding(start = 24.dp))
                script -> ScriptBlock(it, Modifier.padding(start = 24.dp))
                else -> CodeBlock(it, Modifier.padding(start = 24.dp))
            }
        }
        if (result != null) Output(result, open && call.tool !in editTools)
    }
}

/** A long command shown as the terminal does: the first lines, the rest on a tap. */
@Composable
private fun ScriptBlock(text: String, modifier: Modifier = Modifier) {
    var all by remember(text) { mutableStateOf(false) }
    val lines = text.lines()
    val shown = if (all || lines.size <= 18) text else lines.take(15).joinToString("\n")
    Column(modifier) {
        CodeBlock(shown)
        if (!all && lines.size > 18) {
            Text(
                "… ${lines.size - 15} more lines",
                style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.primary,
                modifier = Modifier.clickable { all = true }.padding(start = 10.dp, bottom = 4.dp),
            )
        }
    }
}

/** Removed lines red, added lines green, as the terminal shows an edit. */
@Composable
private fun DiffBlock(text: String, modifier: Modifier = Modifier) {
    val removed = MaterialTheme.colorScheme.error
    val added = Color(0xFF2E7D32)
    Column(
        modifier
            .fillMaxWidth()
            .padding(vertical = 4.dp)
            .background(MaterialTheme.colorScheme.surfaceContainerHigh, RoundedCornerShape(8.dp))
            .padding(10.dp)
            .horizontalScroll(rememberScrollState()),
    ) {
        for (line in text.lines()) {
            Text(
                line,
                fontFamily = FontFamily.Monospace,
                style = MaterialTheme.typography.bodySmall,
                softWrap = false,
                color = when {
                    line.startsWith("- ") -> removed
                    line.startsWith("+ ") -> added
                    else -> MaterialTheme.colorScheme.onSurface
                },
            )
        }
    }
}

/** A tool's output: folded to "⎿ 12 lines" (or the error's first line), open as a block. */
@Composable
private fun Output(result: Item, open: Boolean) {
    val text = result.text.trim()
    val lines = if (text.isEmpty()) 0 else text.count { it == '\n' } + 1
    val color = if (result.error) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurfaceVariant
    Column(Modifier.padding(start = 24.dp)) {
        if (!open) {
            val summary = when {
                text.isEmpty() -> "no output"
                result.error -> "error · " + text.lineSequence().first()
                lines == 1 && text.length <= 60 -> text
                lines == 1 -> "1 line"
                else -> "$lines lines"
            }
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text("⎿", style = MaterialTheme.typography.bodySmall, color = color)
                Spacer(Modifier.width(6.dp))
                Text(
                    summary,
                    fontFamily = FontFamily.Monospace,
                    style = MaterialTheme.typography.bodySmall,
                    color = color,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        } else if (text.isNotEmpty()) {
            Text(
                text,
                fontFamily = FontFamily.Monospace,
                style = MaterialTheme.typography.bodySmall,
                color = color,
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(top = 4.dp)
                    .background(MaterialTheme.colorScheme.surfaceContainerLow, RoundedCornerShape(8.dp))
                    .padding(8.dp)
                    .horizontalScroll(rememberScrollState()),
                softWrap = false,
            )
        }
    }
}

/** A labelled line that opens on a tap. */
@Composable
private fun Folded(label: String, summary: String?, tint: androidx.compose.ui.graphics.Color, content: @Composable () -> Unit) {
    var open by remember(label) { mutableStateOf(false) }
    Column(Modifier.fillMaxWidth().clickable { open = !open }.padding(vertical = 2.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text("✻", color = tint, style = MaterialTheme.typography.bodySmall)
            Spacer(Modifier.width(8.dp))
            Text(label, style = MaterialTheme.typography.labelLarge, color = tint)
            summary?.let {
                Spacer(Modifier.width(8.dp))
                Text(it, style = MaterialTheme.typography.bodySmall, color = tint, maxLines = 1, overflow = TextOverflow.Ellipsis)
            }
            Spacer(Modifier.weight(1f))
            Icon(if (open) Icons.Filled.ExpandLess else Icons.Filled.ExpandMore, null, tint = tint, modifier = Modifier.size(18.dp))
        }
        if (open) content()
    }
}

@Composable
private fun PromptBar(
    text: String,
    onText: (String) -> Unit,
    wrapped: Boolean,
    busy: Boolean,
    waiting: Boolean,
    send: () -> Unit,
    stop: () -> Unit,
) {
    Surface(color = MaterialTheme.colorScheme.surface, tonalElevation = 2.dp) {
        Column(Modifier.navigationBarsPadding().imePadding().padding(horizontal = 12.dp, vertical = 8.dp)) {
            // One line on the session's state, where it changes what Send does.
            val (state, tint) = when {
                waiting -> "Waiting for you · answer the card above" to MaterialTheme.colorScheme.tertiary
                !wrapped -> "Started outside rc2s2 run · Claude sees this as a message from another session · no stop from here" to MaterialTheme.colorScheme.onSurfaceVariant
                busy -> "Claude is working · Send queues after this turn · the stop button interrupts" to MaterialTheme.colorScheme.onSurfaceVariant
                else -> null to MaterialTheme.colorScheme.onSurfaceVariant
            }
            state?.let {
                Text(
                    it,
                    style = MaterialTheme.typography.labelSmall,
                    color = tint,
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.padding(start = 16.dp, bottom = 6.dp),
                )
            }
            Row(
                verticalAlignment = Alignment.Bottom,
                modifier = Modifier
                    .fillMaxWidth()
                    .background(MaterialTheme.colorScheme.surfaceContainerHighest, RoundedCornerShape(24.dp))
                    .padding(start = 18.dp, end = 6.dp, top = 6.dp, bottom = 6.dp),
            ) {
                BasicTextField(
                    value = text,
                    onValueChange = onText,
                    textStyle = MaterialTheme.typography.bodyLarge.copy(color = MaterialTheme.colorScheme.onSurface),
                    cursorBrush = SolidColor(MaterialTheme.colorScheme.primary),
                    maxLines = 6,
                    modifier = Modifier.weight(1f).padding(vertical = 10.dp),
                    decorationBox = { field ->
                        Box {
                            if (text.isEmpty()) {
                                Text(
                                    if (busy) "Queue a prompt" else "Prompt Claude",
                                    style = MaterialTheme.typography.bodyLarge,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                            field()
                        }
                    },
                )
                if (wrapped) {
                    FilledTonalIconButton(onClick = stop, modifier = Modifier.padding(start = 4.dp)) {
                        Icon(Icons.Filled.Stop, if (text.isBlank()) "Interrupt" else "Interrupt, then send")
                    }
                }
                FilledIconButton(onClick = send, enabled = text.isNotBlank(), modifier = Modifier.padding(start = 4.dp)) {
                    Icon(Icons.AutoMirrored.Filled.Send, "Send")
                }
            }
        }
    }
}
