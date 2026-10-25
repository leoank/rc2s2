package me.leoank.rc2s2

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.HelpOutline
import androidx.compose.material.icons.outlined.Lock
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import uniffi.rc2s2_core.PendingInfo
import uniffi.rc2s2_core.Question

/** A request waiting on the person: a permission to allow or deny, or questions to answer.
 *  [where] names the session when the card is shown away from it. */
@Composable
fun PendingCard(model: HubModel, p: PendingInfo, where: String?, modifier: Modifier = Modifier) {
    val question = p.questions.isNotEmpty()
    Card(
        modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.tertiaryContainer),
    ) {
        Column(Modifier.padding(16.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Icon(
                    if (question) Icons.AutoMirrored.Outlined.HelpOutline else Icons.Outlined.Lock,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.onTertiaryContainer,
                )
                Spacer(Modifier.width(8.dp))
                Column(Modifier.weight(1f)) {
                    Text(
                        if (question) "Claude asks" else "Allow ${p.toolName}?",
                        style = MaterialTheme.typography.titleMedium,
                    )
                    val sub = listOfNotNull(where, relative(p.createdMs)).joinToString(" · ")
                    Text(sub, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            }
            if (question) QuestionForm(p.questions) { answers -> model.answer(p, answers) }
            else PermissionForm(p, allow = { model.allow(p) }, deny = { model.deny(p, it) })
        }
    }
}

@Composable
private fun PermissionForm(p: PendingInfo, allow: () -> Unit, deny: (String) -> Unit) {
    var asking by remember(p.id) { mutableStateOf(false) }
    Text(
        p.summary.ifBlank { p.toolInputJson },
        fontFamily = FontFamily.Monospace,
        style = MaterialTheme.typography.bodyMedium,
        color = MaterialTheme.colorScheme.onSurface,
        maxLines = 12,
        overflow = TextOverflow.Ellipsis,
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 12.dp)
            .background(MaterialTheme.colorScheme.surface, RoundedCornerShape(8.dp))
            .padding(10.dp),
    )
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        Button(onClick = allow, modifier = Modifier.weight(1f)) { Text("Allow") }
        OutlinedButton(onClick = { asking = true }, modifier = Modifier.weight(1f)) { Text("Deny") }
    }
    if (asking) {
        var message by remember { mutableStateOf("") }
        AlertDialog(
            onDismissRequest = { asking = false },
            title = { Text("Deny ${p.toolName}") },
            text = {
                Column {
                    Text("Claude sees the reason and can try another way.", style = MaterialTheme.typography.bodyMedium)
                    OutlinedTextField(
                        message, { message = it },
                        label = { Text("Reason (optional)") },
                        modifier = Modifier.fillMaxWidth().padding(top = 12.dp),
                    )
                }
            },
            confirmButton = { TextButton(onClick = { asking = false; deny(message.trim()) }) { Text("Deny") } },
            dismissButton = { TextButton(onClick = { asking = false }) { Text("Cancel") } },
        )
    }
}

/** One block per question: its options as chips, or an answer of the person's own. */
@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun QuestionForm(questions: List<Question>, submit: (Map<String, List<String>>) -> Unit) {
    val chosen = remember(questions) { mutableStateMapOf<String, List<String>>() }
    val own = remember(questions) { mutableStateMapOf<String, String>() }
    for (q in questions) {
        if (q.header.isNotBlank()) {
            Text(
                q.header.uppercase(),
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(top = 12.dp),
            )
        }
        Text(q.question, fontWeight = FontWeight.SemiBold, modifier = Modifier.padding(top = 4.dp, bottom = 4.dp))
        if (q.multiSelect) Caption("Pick any that apply", Modifier.padding(0.dp))
        FlowRow(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
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
                        own[q.question] = ""
                    },
                    label = { Text(option) },
                )
            }
        }
        OutlinedTextField(
            own[q.question].orEmpty(), { own[q.question] = it; if (it.isNotBlank()) chosen.remove(q.question) },
            label = { Text("Or answer in your own words") },
            modifier = Modifier.fillMaxWidth().padding(top = 4.dp),
        )
    }
    val answered = questions.all { q -> own[q.question].orEmpty().isNotBlank() || chosen[q.question].orEmpty().isNotEmpty() }
    Button(
        onClick = {
            submit(questions.associate { q ->
                val typed = own[q.question].orEmpty().trim()
                q.question to if (typed.isNotEmpty()) listOf(typed) else chosen[q.question].orEmpty()
            })
        },
        enabled = answered,
        modifier = Modifier.fillMaxWidth().padding(top = 12.dp),
    ) { Text("Answer") }
}
