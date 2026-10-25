package me.leoank.rc2s2

import android.app.Application
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.receiveAsFlow
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import uniffi.rc2s2_core.Change
import uniffi.rc2s2_core.CoreException
import uniffi.rc2s2_core.HealthPoint
import uniffi.rc2s2_core.Hub
import uniffi.rc2s2_core.Item
import uniffi.rc2s2_core.ModelTotals
import uniffi.rc2s2_core.NodeInfo
import uniffi.rc2s2_core.PendingInfo
import uniffi.rc2s2_core.Role
import uniffi.rc2s2_core.SessionInfo
import uniffi.rc2s2_core.SessionListener
import uniffi.rc2s2_core.Subscription
import uniffi.rc2s2_core.WatchListener

/** One session's conversation as seen so far, with the usage totals and the last seq. */
class SessionCache(val items: ArrayList<Item>, val usage: ArrayList<ModelTotals>, var lastSeq: ULong) {
    /** The cache as JSON; only the last [KEPT] items go to disk. */
    fun snapshot(): String {
        val o = JSONObject()
        o.put("lastSeq", lastSeq.toString())
        o.put("items", JSONArray().apply {
            for (i in items.takeLast(KEPT)) put(JSONObject().apply {
                put("seq", i.seq.toString()); put("timeMs", i.timeMs); put("role", i.role.name)
                put("tool", i.tool); put("text", i.text); put("detail", i.detail ?: JSONObject.NULL); put("error", i.error)
                put("diff", i.diff ?: JSONObject.NULL)
            })
        })
        o.put("usage", JSONArray().apply {
            for (u in usage) put(JSONObject().apply {
                put("model", u.model); put("messages", u.messages.toString()); put("input", u.inputTokens.toString())
                put("output", u.outputTokens.toString()); put("cacheRead", u.cacheReadTokens.toString())
                put("cacheWrite", u.cacheWriteTokens.toString()); put("cost", u.costUsd)
            })
        })
        return o.toString()
    }

    companion object {
        const val KEPT = 2000

        fun load(file: File): SessionCache {
            val empty = SessionCache(ArrayList(), ArrayList(), 0uL)
            if (!file.exists()) return empty
            return runCatching {
                val o = JSONObject(file.readText())
                val items = ArrayList<Item>()
                val arr = o.getJSONArray("items")
                for (k in 0 until arr.length()) {
                    val i = arr.getJSONObject(k)
                    items.add(Item(
                        seq = i.getString("seq").toULong(), timeMs = i.getLong("timeMs"), role = Role.valueOf(i.getString("role")),
                        tool = i.optString("tool"), text = i.optString("text"),
                        detail = if (i.isNull("detail")) null else i.getString("detail"), error = i.optBoolean("error"),
                        diff = if (i.isNull("diff")) null else i.getString("diff"),
                    ))
                }
                val usage = ArrayList<ModelTotals>()
                val us = o.getJSONArray("usage")
                for (k in 0 until us.length()) {
                    val u = us.getJSONObject(k)
                    usage.add(ModelTotals(
                        model = u.getString("model"), messages = u.getString("messages").toULong(),
                        inputTokens = u.getString("input").toULong(), outputTokens = u.getString("output").toULong(),
                        cacheReadTokens = u.getString("cacheRead").toULong(), cacheWriteTokens = u.getString("cacheWrite").toULong(),
                        costUsd = u.getDouble("cost"),
                    ))
                }
                SessionCache(items, usage, o.getString("lastSeq").toULong())
            }.getOrDefault(empty)
        }
    }
}

/** A prompt sent from this phone, until the transcript echoes it. */
data class Sent(
    val text: String,
    val timeMs: Long,
    val accepted: Boolean = false,
    val note: String? = null,
    val failed: String? = null,
)

/** Where the connection to the hub stands. */
enum class Link { Disconnected, Connecting, Connected, Reconnecting }

/** The app's whole state: the hub connection from the core, and what it lists and streams.
 *  The core's listeners call from its own threads; every change is applied on the main
 *  thread, where Compose reads it. A broken watch reconnects by itself with backoff, and
 *  an open session's stream resumes after the last item seen. */
class HubModel(app: Application) : AndroidViewModel(app) {
    private val prefs = app.getSharedPreferences("rc2s2", 0)

    var address by mutableStateOf(prefs.getString("address", null) ?: "http://ghost001:7700")
    var token by mutableStateOf(prefs.getString("token", null) ?: "")
    var link by mutableStateOf(Link.Disconnected)
        private set
    /** Why the last connection attempt failed, for the connect screen. */
    var connectError by mutableStateOf<String?>(null)
        private set

    /** One-off outcomes to show briefly: a refusal's reason, a confirmation, an error. */
    private val notices = Channel<String>(Channel.BUFFERED)
    val messages = notices.receiveAsFlow()

    val nodes = mutableStateListOf<NodeInfo>()
    val sessions = mutableStateListOf<SessionInfo>()
    val pending = mutableStateListOf<PendingInfo>()

    /** The session whose conversation is streamed, and the conversation itself. */
    var openSession by mutableStateOf<SessionInfo?>(null)
        private set
    val conversation = mutableStateListOf<Item>()
    /** What was seen of each session, kept in memory while the app lives and on disk
     *  between runs, so opening a session shows it at once and the stream resumes after
     *  the last item instead of replaying everything (phase 1.5 item 14). */
    private val caches = HashMap<String, SessionCache>()
    private val cacheDir = File(app.filesDir, "sessions").apply { mkdirs() }
    /** Prompts sent from here that the transcript has not echoed yet, newest last. */
    val sending = mutableStateListOf<Sent>()
    /** Texts the transcript echoed after being sent from here: shown with two ticks. */
    val delivered = mutableStateListOf<String>()
    /** The open session's token totals by model, from the daemon's count. */
    val usage = mutableStateListOf<ModelTotals>()
    private var lastSeq = 0uL

    val health = mutableStateListOf<HealthPoint>()
    var healthLoading by mutableStateOf(false)
        private set

    private var hub: Hub? = null
    private var watch: Subscription? = null
    private var stream: Subscription? = null
    private var reconnect: Job? = null

    init {
        if (token.isNotBlank()) connect()
    }

    /** The hub's host, for titles. */
    val hubName: String
        get() = address.trim().removePrefix("http://").removePrefix("https://").substringBefore('/')

    fun connect() {
        prefs.edit().putString("address", address.trim()).putString("token", token.trim()).apply()
        reconnect?.cancel()
        viewModelScope.launch {
            link = Link.Connecting
            connectError = null
            try {
                establish()
            } catch (e: Exception) {
                connectError = describe(e)
                link = Link.Disconnected
            }
        }
    }

    fun disconnect() {
        reconnect?.cancel()
        watch?.cancel(); watch = null
        stream?.cancel(); stream = null
        hub?.close(); hub = null
        link = Link.Disconnected
        nodes.clear(); sessions.clear(); pending.clear()
        openSession = null
        conversation.clear()
    }

    /** Opens the connection, lists everything, and starts the watch and any open stream.
     *  The old hub object goes first: a command or refresh while the new connection is
     *  being made must see "not connected", not a destroyed object. */
    private suspend fun establish() {
        watch?.cancel()
        stream?.cancel()
        hub?.let { old -> hub = null; old.close() }
        val h = Hub.connect(address.trim(), token.trim())
        hub = h
        reload(h)
        watch = h.watch(object : WatchListener {
            override fun onChange(change: Change) = onMain { apply(change) }
            override fun onError(message: String) = onMain { lost(message) }
        })
        openSession?.let { startStream(h, it, lastSeq) }
        link = Link.Connected
    }

    /** The watch ended: try again until it holds, with a short backoff. The core's
     *  keepalive pings end a dead link within half a minute, so this is also how a
     *  phone's silent drop (sleep, a network change) is recovered. */
    private fun lost(message: String) {
        if (link == Link.Disconnected || reconnect?.isActive == true) return
        link = Link.Reconnecting
        reconnect = viewModelScope.launch {
            var wait = 1_000L
            while (isActive) {
                try {
                    establish()
                    return@launch
                } catch (e: Exception) {
                    connectError = describe(e)
                }
                delay(wait)
                wait = (wait * 2).coerceAtMost(10_000)
            }
        }
    }

    /** Called when the app comes to the front. A reconnect in progress retries at once
     *  instead of waiting out its backoff; a connected link is checked with a list, and a
     *  failure there starts the reconnect without waiting for the keepalive. */
    fun resumed() {
        when (link) {
            Link.Reconnecting -> {
                reconnect?.cancel()
                reconnect = null
                lost("resumed")
            }
            Link.Connected -> viewModelScope.launch {
                val h = hub ?: return@launch
                try {
                    reload(h)
                } catch (e: CoreException.Connection) {
                    lost(e.reason)
                } catch (e: Exception) {
                    say(describe(e))
                }
            }
            else -> {}
        }
    }

    fun refresh() = withHub { reload(it) }

    private suspend fun reload(h: Hub) {
        val n = h.nodes()
        val s = h.sessions(false)
        val p = h.pending()
        nodes.replaceAll(n)
        sessions.replaceAll(s)
        pending.replaceAll(p)
        openSession?.let { open -> sessions.firstOrNull { it.id == open.id && it.nodeId == open.nodeId }?.let { openSession = it } }
    }

    private fun apply(change: Change) {
        when (change) {
            is Change.Node -> nodes.upsert(change.node) { it.id == change.node.id }
            is Change.Session -> {
                val s = change.session
                if (s.endedMs != null) sessions.removeAll { it.id == s.id && it.nodeId == s.nodeId }
                else sessions.upsert(s) { it.id == s.id && it.nodeId == s.nodeId }
                if (openSession?.id == s.id && openSession?.nodeId == s.nodeId) openSession = s
            }
            is Change.PendingOpened -> pending.upsert(change.pending) { it.id == change.pending.id }
            is Change.PendingClosed -> pending.removeAll { it.id == change.pendingId }
        }
    }

    fun pendingFor(s: SessionInfo): List<PendingInfo> = pending.filter { it.sessionId == s.id && it.nodeId == s.nodeId }

    fun node(id: String): NodeInfo? = nodes.firstOrNull { it.id == id }

    fun allow(p: PendingInfo) = withHub { done(it.allow(p.nodeId, p.id), "Allowed") }
    fun deny(p: PendingInfo, message: String) = withHub { done(it.deny(p.nodeId, p.id, message), "Denied") }
    fun answer(p: PendingInfo, answers: Map<String, List<String>>) =
        withHub { done(it.answer(p.nodeId, p.id, answers), "Answered") }

    fun prompt(s: SessionInfo, text: String) = send(s, text) { it.prompt(s.nodeId, s.id, text) }
    fun interrupt(s: SessionInfo, then: String?) {
        val follow = then?.takeIf { t -> t.isNotBlank() }
        if (follow == null) withHub { done(it.interrupt(s.nodeId, s.id, null), "Interrupted") }
        else send(s, follow) { it.interrupt(s.nodeId, s.id, follow) }
    }

    /** A failed send, sent again as a new attempt. */
    fun retry(x: Sent) {
        val s = openSession ?: return
        sending.remove(x)
        prompt(s, x.text)
    }

    /** A prompt from here appears at once with one tick; the hub's answer sets the note or
     *  the failure; the transcript's echo turns it into the real turn with two ticks. */
    private fun send(s: SessionInfo, text: String, call: suspend (Hub) -> String) {
        val sent = Sent(text, System.currentTimeMillis())
        sending.add(sent)
        viewModelScope.launch {
            val h = hub
            val i = sending.indexOf(sent)
            if (h == null) {
                if (i >= 0) sending[i] = sent.copy(failed = "not connected")
                return@launch
            }
            try {
                val note = call(h)
                val j = sending.indexOf(sent)
                if (j >= 0) sending[j] = sent.copy(accepted = true, note = note.ifEmpty { null })
            } catch (e: Exception) {
                val j = sending.indexOf(sent)
                if (j >= 0) sending[j] = sent.copy(failed = describe(e))
            }
        }
    }
    fun kill(s: SessionInfo) = withHub { done(it.kill(s.nodeId, s.id), "Killed") }

    /** Shows one session's conversation: what the cache has at once, then the stream from
     *  where the cache ends. */
    fun open(s: SessionInfo) {
        if (openSession?.id == s.id && openSession?.nodeId == s.nodeId && stream != null) return
        openSession = s
        conversation.clear()
        sending.clear()
        delivered.clear()
        usage.clear()
        val cache = caches.getOrPut(key(s)) { SessionCache.load(File(cacheDir, key(s) + ".json")) }
        conversation.addAll(cache.items)
        usage.addAll(cache.usage)
        lastSeq = cache.lastSeq
        withHub { startStream(it, s, cache.lastSeq) }
    }

    private fun key(s: SessionInfo) = "${s.nodeId}_${s.id}"

    /** While a session opens, its stored items are gathered here until the stream goes
     *  quiet, then shown in one piece, so the list lands at the end instead of scrolling
     *  through the history as it arrives. */
    private var loading = false
    private val backlog = ArrayList<Item>()

    private suspend fun startStream(h: Hub, s: SessionInfo, after: ULong) {
        stream?.cancel()
        val cache = caches[key(s)]
        if (after == 0uL) {
            loading = true
            backlog.clear()
            viewModelScope.launch {
                var seen = -1
                while (loading && openSession?.id == s.id) {
                    delay(150)
                    if (backlog.size == seen) break
                    seen = backlog.size
                }
                loading = false
                if (openSession?.id == s.id) {
                    conversation.addAll(backlog)
                    backlog.clear()
                }
            }
        }
        stream = h.streamSession(s.nodeId, s.id, after, cache?.usage?.toList() ?: emptyList(), object : SessionListener {
            override fun onItems(items: List<Item>, lastSeq: ULong) = onMain {
                cache?.let { c -> c.items.addAll(items); c.lastSeq = lastSeq }
                if (openSession?.id != s.id) return@onMain
                for (item in items) {
                    if (item.role == Role.USER) {
                        val i = sending.indexOfFirst { it.text == item.text }
                        if (i >= 0) {
                            sending.removeAt(i)
                            delivered.add(item.text)
                        }
                    }
                }
                this@HubModel.lastSeq = lastSeq
                if (loading) backlog.addAll(items) else conversation.addAll(items)
            }
            override fun onUsage(totals: List<ModelTotals>) = onMain {
                cache?.let { c -> c.usage.clear(); c.usage.addAll(totals) }
                if (openSession?.id != s.id) return@onMain
                usage.clear()
                usage.addAll(totals)
            }
            override fun onError(message: String) = onMain {
                if (openSession?.id == s.id) say("Conversation stream ended: $message")
            }
        })
    }

    fun close() {
        stream?.cancel()
        stream = null
        openSession?.let { s -> caches[key(s)]?.let { c -> persist(key(s), c) } }
        openSession = null
        loading = false
        backlog.clear()
        conversation.clear()
        sending.clear()
        delivered.clear()
        usage.clear()
    }

    fun loadHealth(nodeId: String, sinceMs: Long) = withHub {
        healthLoading = true
        try {
            health.replaceAll(it.health(nodeId, sinceMs))
        } finally {
            healthLoading = false
        }
    }

    private fun persist(key: String, cache: SessionCache) {
        val snapshot = cache.snapshot()
        viewModelScope.launch(Dispatchers.IO) {
            runCatching { File(cacheDir, "$key.json").writeText(snapshot) }
        }
    }

    fun say(text: String) {
        notices.trySend(text)
    }

    /** A command's outcome: the node's note when it has one, else a short confirmation. */
    private fun done(note: String, fallback: String) = say(note.ifEmpty { fallback })

    private fun withHub(block: suspend (Hub) -> Unit) {
        viewModelScope.launch {
            val h = hub
            if (h == null) {
                if (link == Link.Reconnecting) say("Reconnecting to the hub, try again in a moment")
                else if (link == Link.Connected) say("Not connected")
                return@launch
            }
            try {
                block(h)
            } catch (e: Exception) {
                say(describe(e))
            }
        }
    }

    private fun describe(e: Exception): String = when (e) {
        is CoreException.Connection -> "Cannot reach the hub: ${e.reason}"
        is CoreException.Hub -> e.reason
        is CoreException.Refused -> "The node refused: ${e.reason}"
        else -> e.message ?: e.toString()
    }

    private fun onMain(block: () -> Unit) {
        viewModelScope.launch(Dispatchers.Main) { block() }
    }

    override fun onCleared() {
        openSession?.let { s -> caches[key(s)]?.let { c -> runCatching { File(cacheDir, key(s) + ".json").writeText(c.snapshot()) } } }
        reconnect?.cancel()
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
