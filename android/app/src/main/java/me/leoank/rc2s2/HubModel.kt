package me.leoank.rc2s2

import android.app.Application
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import uniffi.rc2s2_core.Change
import uniffi.rc2s2_core.CoreException
import uniffi.rc2s2_core.HealthPoint
import uniffi.rc2s2_core.Hub
import uniffi.rc2s2_core.Item
import uniffi.rc2s2_core.NodeInfo
import uniffi.rc2s2_core.PendingInfo
import uniffi.rc2s2_core.SessionInfo
import uniffi.rc2s2_core.SessionListener
import uniffi.rc2s2_core.Subscription
import uniffi.rc2s2_core.WatchListener

/** The app's whole state: the hub connection from the core, and what it lists and streams.
 *  The core's listeners call from its own threads; every change is applied on the main
 *  thread, where Compose reads it. */
class HubModel(app: Application) : AndroidViewModel(app) {
    private val prefs = app.getSharedPreferences("rc2s2", 0)

    var address by mutableStateOf(prefs.getString("address", null) ?: "http://10.0.2.2:7700")
    var token by mutableStateOf(prefs.getString("token", null) ?: "")
    var connected by mutableStateOf(false)
        private set
    /** The last outcome to show the person: an error, a refusal's reason, or a note. */
    var notice by mutableStateOf<String?>(null)

    val nodes = mutableStateListOf<NodeInfo>()
    val sessions = mutableStateListOf<SessionInfo>()
    val pending = mutableStateListOf<PendingInfo>()
    val conversation = mutableStateListOf<Item>()
    val health = mutableStateListOf<HealthPoint>()

    private var hub: Hub? = null
    private var watch: Subscription? = null
    private var stream: Subscription? = null

    fun connect() = act {
        prefs.edit().putString("address", address).putString("token", token).apply()
        val h = Hub.connect(address.trim(), token.trim())
        hub = h
        watch?.cancel()
        reload(h)
        watch = h.watch(object : WatchListener {
            override fun onChange(change: Change) = onMain { apply(change) }
            override fun onError(message: String) = onMain {
                notice = "Watch ended: $message"
                connected = false
            }
        })
        connected = true
    }

    fun refresh() = withHub { reload(it) }

    private suspend fun reload(h: Hub) {
        val n = h.nodes()
        val s = h.sessions(false)
        val p = h.pending()
        nodes.replaceAll(n)
        sessions.replaceAll(s)
        pending.replaceAll(p)
    }

    private fun apply(change: Change) {
        when (change) {
            is Change.Node -> nodes.upsert(change.node) { it.id == change.node.id }
            is Change.Session -> {
                val s = change.session
                if (s.endedMs != null) sessions.removeAll { it.id == s.id }
                else sessions.upsert(s) { it.id == s.id && it.nodeId == s.nodeId }
            }
            is Change.PendingOpened -> pending.upsert(change.pending) { it.id == change.pending.id }
            is Change.PendingClosed -> pending.removeAll { it.id == change.pendingId }
        }
    }

    fun allow(p: PendingInfo) = withHub { note(it.allow(p.nodeId, p.id)) }
    fun deny(p: PendingInfo, message: String) = withHub { note(it.deny(p.nodeId, p.id, message)) }
    fun answer(p: PendingInfo, answers: Map<String, List<String>>) =
        withHub { note(it.answer(p.nodeId, p.id, answers)) }

    fun prompt(s: SessionInfo, text: String) = withHub { note(it.prompt(s.nodeId, s.id, text)) }
    fun interrupt(s: SessionInfo, then: String?) =
        withHub { note(it.interrupt(s.nodeId, s.id, then?.takeIf { t -> t.isNotBlank() })) }
    fun kill(s: SessionInfo) = withHub { note(it.kill(s.nodeId, s.id)) }

    /** Streams one session's conversation into [conversation], replacing any earlier one. */
    fun open(s: SessionInfo) = withHub { h ->
        stream?.cancel()
        conversation.clear()
        stream = h.streamSession(s.nodeId, s.id, 0uL, object : SessionListener {
            override fun onItems(items: List<Item>, lastSeq: ULong) = onMain { conversation.addAll(items) }
            override fun onError(message: String) = onMain { notice = "Stream ended: $message" }
        })
    }

    fun close() {
        stream?.cancel()
        stream = null
    }

    fun loadHealth(nodeId: String) = withHub {
        val hour = System.currentTimeMillis() - 3_600_000
        health.replaceAll(it.health(nodeId, hour))
    }

    private fun note(n: String) {
        notice = n.ifEmpty { null }
    }

    private fun withHub(block: suspend (Hub) -> Unit) = act {
        val h = hub ?: throw IllegalStateException("Not connected")
        block(h)
    }

    private fun act(block: suspend () -> Unit) {
        viewModelScope.launch {
            try {
                block()
            } catch (e: CoreException) {
                notice = when (e) {
                    is CoreException.Connection -> "Cannot reach the hub: ${e.reason}"
                    is CoreException.Hub -> e.reason
                    is CoreException.Refused -> "The node refused: ${e.reason}"
                }
            } catch (e: Exception) {
                notice = e.message ?: e.toString()
            }
        }
    }

    private fun onMain(block: () -> Unit) {
        viewModelScope.launch(Dispatchers.Main) { block() }
    }

    override fun onCleared() {
        watch?.cancel()
        stream?.cancel()
        hub?.close()
    }
}

private fun <T> MutableList<T>.replaceAll(items: List<T>) {
    clear()
    addAll(items)
}

private fun <T> MutableList<T>.upsert(item: T, same: (T) -> Boolean) {
    val i = indexOfFirst(same)
    if (i >= 0) this[i] = item else add(item)
}
