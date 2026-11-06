package me.leoank.rc2s2

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import uniffi.rc2s2_core.NodeInfo

/** Models offered for a new session: claude's `--model` aliases, empty for its default. */
private val models = listOf("Default" to "", "Fable" to "fable", "Opus" to "opus", "Sonnet" to "sonnet", "Haiku" to "haiku")

/** A new headless session (ADR-0034): a node that allows spawning, a directory under one of
 *  its roots (made if missing, unless switched off), an optional first prompt and a model.
 *  [start] gets the node's id and the choices. */
@OptIn(ExperimentalLayoutApi::class)
@Composable
fun NewSessionDialog(
    model: HubModel,
    nodes: List<NodeInfo>,
    dismiss: () -> Unit,
    start: (nodeId: String, cwd: String, prompt: String, model: String, create: Boolean) -> Unit,
) {
    var nodeId by remember { mutableStateOf(nodes.first().id) }
    val node = nodes.firstOrNull { it.id == nodeId } ?: nodes.first()
    val dirs = model.spawnDirs(node)
    var cwd by remember(nodeId) { mutableStateOf(dirs.firstOrNull() ?: "") }
    var prompt by remember { mutableStateOf("") }
    var chosen by remember { mutableStateOf("") }
    var create by remember { mutableStateOf(true) }
    AlertDialog(
        onDismissRequest = dismiss,
        title = { Text("New session") },
        text = {
            Column(
                verticalArrangement = Arrangement.spacedBy(8.dp),
                modifier = Modifier.verticalScroll(rememberScrollState()),
            ) {
                if (nodes.size > 1) {
                    Label("Node")
                    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        for (n in nodes) FilterChip(selected = n.id == nodeId, onClick = { nodeId = n.id }, label = { Text(n.name) })
                    }
                }
                Label("Directory")
                OutlinedTextField(
                    value = cwd,
                    onValueChange = { cwd = it },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                    supportingText = { Text("Under ${node.spawnRoots.joinToString(" or ")}") },
                )
                FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    for (d in dirs) FilterChip(selected = d == cwd, onClick = { cwd = d }, label = { Text(d.substringAfterLast('/')) })
                }
                Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.fillMaxWidth()) {
                    Text("Create it if it does not exist", style = MaterialTheme.typography.bodyMedium, modifier = Modifier.weight(1f))
                    Switch(checked = create, onCheckedChange = { create = it })
                }
                Label("First prompt (optional)")
                OutlinedTextField(
                    value = prompt,
                    onValueChange = { prompt = it },
                    minLines = 3,
                    maxLines = 8,
                    placeholder = { Text("Leave empty to start idle and type in the session") },
                    modifier = Modifier.fillMaxWidth(),
                )
                Label("Model")
                FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    for ((label, alias) in models) FilterChip(selected = chosen == alias, onClick = { chosen = alias }, label = { Text(label) })
                }
                Text(
                    "Runs headless with this node's own Claude login. It cannot ask questions; its permission requests come here.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(top = 4.dp),
                )
            }
        },
        confirmButton = {
            TextButton(
                enabled = cwd.isNotBlank(),
                onClick = { start(node.id, cwd, prompt, chosen, create) },
            ) { Text("Start") }
        },
        dismissButton = { TextButton(onClick = dismiss) { Text("Cancel") } },
    )
}

@Composable
private fun Label(text: String) {
    Text(text, style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
}
