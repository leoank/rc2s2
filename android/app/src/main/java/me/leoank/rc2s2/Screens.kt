package me.leoank.rc2s2

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import uniffi.rc2s2_core.Item
import uniffi.rc2s2_core.NodeInfo
import uniffi.rc2s2_core.PendingInfo
import uniffi.rc2s2_core.Question
import uniffi.rc2s2_core.Role
import uniffi.rc2s2_core.SessionInfo

@Composable
private fun Page(title: String, model: HubModel, content: @Composable ColumnScope.() -> Unit) {
    Column(
        Modifier
            .fillMaxSize()
            .background(MaterialTheme.colorScheme.background)
            .safeDrawingPadding()
            .padding(horizontal = 16.dp)
    ) {
        Text(title, style = MaterialTheme.typography.headlineSmall, modifier = Modifier.padding(vertical = 12.dp))
        model.notice?.let { n ->
            Card(Modifier.fillMaxWidth().padding(bottom = 8.dp).clickable { model.notice = null }) {
                Text(n, Modifier.padding(12.dp), style = MaterialTheme.typography.bodyMedium)
            }
        }
        content()
    }
}

@Composable
fun ConnectScreen(model: HubModel) {
    Page("rc2s2", model) {
        OutlinedTextField(model.address, { model.address = it }, label = { Text("Hub address") },
            singleLine = true, modifier = Modifier.fillMaxWidth())
        Spacer(Modifier.height(8.dp))
        OutlinedTextField(model.token, { model.token = it }, label = { Text("Client token") },
            singleLine = true, modifier = Modifier.fillMaxWidth())
        Spacer(Modifier.height(16.dp))
        Button(onClick = { model.connect() }, modifier = Modifier.fillMaxWidth()) { Text("Connect") }
    }
}

@Composable
fun HomeScreen(model: HubModel, openSession: (SessionInfo) -> Unit, openHealth: (NodeInfo) -> Unit) {
    Page("Sessions", model) {
        LazyColumn(verticalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.fillMaxSize()) {
            if (model.pending.isNotEmpty()) {
                item { Section("Waiting for you") }
                items(model.pending, key = { "p" + it.id }) { p -> PendingCard(model, p) }
            }
            for (node in model.nodes) {
                item(key = "n" + node.id) { NodeRow(node) { openHealth(node) } }
                val sessions = model.sessions.filter { it.nodeId == node.id }
                if (sessions.isEmpty()) item(key = "e" + node.id) { Faint("No live sessions") }
                items(sessions, key = { "s" + it.nodeId + it.id }) { s -> SessionRow(s) { openSession(s) } }
            }
            item { TextButton(onClick = { model.refresh() }) { Text("Refresh") } }
        }
    }
}

@Composable
private fun Section(text: String) {
    Text(text, style = MaterialTheme.typography.titleSmall, color = MaterialTheme.colorScheme.primary,
        modifier = Modifier.padding(top = 8.dp))
}

@Composable
private fun Faint(text: String) {
    Text(text, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant,
        modifier = Modifier.padding(start = 16.dp))
}

@Composable
private fun NodeRow(node: NodeInfo, onHealth: () -> Unit) {
    Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.fillMaxWidth().padding(top = 12.dp)) {
        Box(Modifier.size(10.dp).background(if (node.connected) Color(0xFF2E7D32) else Color.Gray, CircleShape))
        Spacer(Modifier.width(8.dp))
        Text(node.name, style = MaterialTheme.typography.titleMedium, modifier = Modifier.weight(1f))
        TextButton(onClick = onHealth) { Text("Health") }
    }
}

@Composable
private fun SessionRow(s: SessionInfo, onOpen: () -> Unit) {
    Card(Modifier.fillMaxWidth().clickable(onClick = onOpen)) {
        Column(Modifier.padding(12.dp)) {
            Text(s.cwd.substringAfterLast('/').ifEmpty { s.cwd }, style = MaterialTheme.typography.titleSmall)
            val tags = listOfNotNull(
                if (s.busy) "busy" else "idle",
                if (s.wrapped) "wrapped" else null,
                if (s.pid != 0u) "pid ${s.pid}" else null,
            )
            Text(tags.joinToString(" · "), style = MaterialTheme.typography.bodySmall)
        }
    }
}

@Composable
private fun PendingCard(model: HubModel, p: PendingInfo) {
    val node = model.nodes.firstOrNull { it.id == p.nodeId }?.name ?: "node"
    Card(Modifier.fillMaxWidth()) {
        Column(Modifier.padding(12.dp)) {
            Text("$node · ${p.toolName}", style = MaterialTheme.typography.labelLarge)
            if (p.questions.isEmpty()) {
                Text(p.summary, fontFamily = FontFamily.Monospace, modifier = Modifier.padding(vertical = 8.dp))
                var message by remember(p.id) { mutableStateOf("") }
                OutlinedTextField(message, { message = it }, label = { Text("Message if denied") },
                    singleLine = true, modifier = Modifier.fillMaxWidth())
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.padding(top = 8.dp)) {
                    Button(onClick = { model.allow(p) }) { Text("Allow") }
                    OutlinedButton(onClick = { model.deny(p, message) }) { Text("Deny") }
                }
            } else {
                QuestionForm(p.questions) { answers -> model.answer(p, answers) }
            }
        }
    }
}

/** One block per question: its options as chips, or an answer of the person's own. */
@Composable
private fun QuestionForm(questions: List<Question>, submit: (Map<String, List<String>>) -> Unit) {
    val chosen = remember(questions) { mutableStateMapOf<String, List<String>>() }
    val own = remember(questions) { mutableStateMapOf<String, String>() }
    for (q in questions) {
        Text(q.question, fontWeight = FontWeight.SemiBold, modifier = Modifier.padding(top = 8.dp))
        Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            for (option in q.options) {
                val picked = chosen[q.question].orEmpty()
                FilterChip(
                    selected = option in picked,
                    onClick = {
                        chosen[q.question] = when {
                            option in picked -> picked - option
                            q.multiSelect -> picked + option
                            else -> listOf(option)
                        }
                    },
                    label = { Text(option) },
                )
            }
        }
        OutlinedTextField(own[q.question].orEmpty(), { own[q.question] = it }, label = { Text("Or your own answer") },
            singleLine = true, modifier = Modifier.fillMaxWidth())
    }
    Button(
        onClick = {
            submit(questions.associate { q ->
                val typed = own[q.question].orEmpty().trim()
                q.question to if (typed.isNotEmpty()) listOf(typed) else chosen[q.question].orEmpty()
            })
        },
        modifier = Modifier.padding(top = 8.dp),
    ) { Text("Answer") }
}

@Composable
fun SessionScreen(model: HubModel, s: SessionInfo) {
    Page(s.cwd.substringAfterLast('/').ifEmpty { "Session" }, model) {
        val list = rememberLazyListState()
        LaunchedEffect(model.conversation.size) {
            if (model.conversation.isNotEmpty()) list.animateScrollToItem(model.conversation.size - 1)
        }
        LazyColumn(state = list, modifier = Modifier.weight(1f).fillMaxWidth(),
            verticalArrangement = Arrangement.spacedBy(6.dp)) {
            items(model.conversation, key = { "${it.seq}-${it.role}-${it.text.hashCode()}" }) { ConversationItem(it) }
        }
        var text by remember { mutableStateOf("") }
        Column(Modifier.imePadding().padding(vertical = 8.dp)) {
            OutlinedTextField(text, { text = it }, label = { Text("Prompt") }, modifier = Modifier.fillMaxWidth())
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.padding(top = 8.dp)) {
                Button(onClick = { model.prompt(s, text); text = "" }, enabled = text.isNotBlank()) { Text("Send") }
                if (s.wrapped) {
                    OutlinedButton(onClick = { model.interrupt(s, text); text = "" }) {
                        Text(if (text.isBlank()) "Interrupt" else "Interrupt, then send")
                    }
                    OutlinedButton(onClick = { model.kill(s) }) { Text("Kill") }
                }
            }
        }
    }
}

@Composable
private fun ConversationItem(item: Item) {
    val (label, tint) = when (item.role) {
        Role.USER -> "You" to MaterialTheme.colorScheme.primary
        Role.ASSISTANT -> "Claude" to MaterialTheme.colorScheme.tertiary
        Role.TOOL -> "Tool" to MaterialTheme.colorScheme.secondary
        Role.TOOL_RESULT -> "Result" to MaterialTheme.colorScheme.onSurfaceVariant
        Role.SYSTEM -> "Claude Code" to MaterialTheme.colorScheme.onSurfaceVariant
        Role.DAEMON -> "rc2s2" to MaterialTheme.colorScheme.error
    }
    Column(Modifier.fillMaxWidth()) {
        Text(label, style = MaterialTheme.typography.labelSmall, color = tint)
        val mono = item.role == Role.TOOL || item.role == Role.TOOL_RESULT
        Text(item.text, style = MaterialTheme.typography.bodyMedium,
            fontFamily = if (mono) FontFamily.Monospace else FontFamily.Default)
    }
}

/** The metrics charted per node, with how to scale each to a percentage. */
private val charts = listOf(
    "CPU" to { m: Map<String, Double> -> m["cpu.util_pct"] },
    "Memory" to { m: Map<String, Double> ->
        val used = m["mem.used_bytes"]; val total = m["mem.total_bytes"]
        if (used != null && total != null && total > 0) 100 * used / total else null
    },
    "GPU" to { m: Map<String, Double> -> m["gpu.util_pct"] },
)

@Composable
fun HealthScreen(model: HubModel, node: NodeInfo) {
    Page("${node.name} health", model) {
        if (model.health.isEmpty()) Faint("No samples in the last hour")
        val points = model.health.map { p -> p.timeMs to p.metrics.associate { it.name to it.value } }
        for ((name, pick) in charts) {
            val series = points.mapNotNull { (t, m) -> pick(m)?.let { t to it } }
            if (series.isEmpty()) continue
            Text("$name  ${"%.0f".format(series.last().second)}%", style = MaterialTheme.typography.titleSmall,
                modifier = Modifier.padding(top = 12.dp))
            LineChart(series, Modifier.fillMaxWidth().height(96.dp))
        }
        TextButton(onClick = { model.loadHealth(node.id) }) { Text("Refresh") }
    }
}

/** A 0–100 % line over time. */
@Composable
private fun LineChart(series: List<Pair<Long, Double>>, modifier: Modifier) {
    val color = MaterialTheme.colorScheme.primary
    val grid = MaterialTheme.colorScheme.outlineVariant
    Canvas(modifier.padding(vertical = 4.dp)) {
        drawLine(grid, start = androidx.compose.ui.geometry.Offset(0f, size.height),
            end = androidx.compose.ui.geometry.Offset(size.width, size.height))
        if (series.size < 2) return@Canvas
        val t0 = series.first().first
        val span = (series.last().first - t0).coerceAtLeast(1).toFloat()
        val path = Path()
        series.forEachIndexed { i, (t, v) ->
            val x = (t - t0) / span * size.width
            val y = size.height - (v.coerceIn(0.0, 100.0) / 100.0).toFloat() * size.height
            if (i == 0) path.moveTo(x, y) else path.lineTo(x, y)
        }
        drawPath(path, color, style = Stroke(width = 2.dp.toPx()))
    }
}
