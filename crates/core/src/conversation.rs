//! A session's events as a conversation a person reads: what they asked, what Claude said,
//! thought and did, and what the daemon knows. The transcript carries the conversation;
//! hooks mostly repeat it and are shown only where they add something (EXP-0002, EXP-0010).
//! Every shape Claude Code writes into a user line (slash commands and their output, `!`
//! shell commands, background-task notices, system reminders, peer messages) is turned into
//! something readable or dropped, so the client never shows raw markup.

use rc2s2_proto::event::v1::{Event, EventSource};
use serde_json::Value;

/// How much of a tool result, a thinking block or a tool's detail is kept: the client folds
/// them until tapped, so a screen or two is enough.
const RESULT_CHARS: usize = 4000;
const THINKING_CHARS: usize = 2000;
const DETAIL_CHARS: usize = 4000;
/// The one-line input of a tool call.
const HEADLINE_CHARS: usize = 200;

#[derive(Debug, Clone, PartialEq, uniffi::Enum)]
pub enum Role {
    User,
    Assistant,
    /// Claude's thinking, folded by default.
    Thinking,
    /// Claude calling a tool: `tool` names it, `text` is the one input a person cares about,
    /// `detail` the rest worth a look (an edit's diff, a file's content, an agent's prompt).
    Tool,
    /// What a tool returned, clipped; `error` when the tool failed.
    ToolResult,
    /// Claude Code's own notices: slash commands, compaction, held messages, notifications.
    System,
    /// Facts only the daemon knows, such as an interrupted turn.
    Daemon,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Item {
    pub seq: u64,
    pub time_ms: i64,
    pub role: Role,
    /// The tool's short name for a `Tool` item; empty otherwise.
    pub tool: String,
    pub text: String,
    /// What a tap reveals, when there is more than the headline.
    pub detail: Option<String>,
    pub error: bool,
}

/// The items one event shows as; most events show as none or one.
pub fn items(e: &Event) -> Vec<Item> {
    let v: Value = serde_json::from_str(&e.payload_json).unwrap_or_default();
    let item = |role, text: String| Item {
        seq: e.seq,
        time_ms: e.time_ms,
        role,
        tool: String::new(),
        text,
        detail: None,
        error: false,
    };
    let mut out = Vec::new();
    match e.source() {
        EventSource::Transcript => match e.kind.as_str() {
            "user" => {
                let meta = v["isMeta"].as_bool().unwrap_or(false);
                match &v["message"]["content"] {
                    Value::String(s) => out.extend(user_text(s, meta).map(|(r, t)| item(r, t))),
                    Value::Array(blocks) => {
                        for b in blocks {
                            match b["type"].as_str() {
                                Some("text") => {
                                    out.extend(user_text(&text(&b["text"]), meta).map(|(r, t)| item(r, t)))
                                }
                                Some("tool_result") => {
                                    let mut i = item(Role::ToolResult, clip(&result_text(&b["content"]), RESULT_CHARS));
                                    i.error = b["is_error"].as_bool().unwrap_or(false);
                                    out.push(i);
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
            "assistant" => {
                for b in v["message"]["content"].as_array().into_iter().flatten() {
                    match b["type"].as_str() {
                        Some("text") if !text(&b["text"]).trim().is_empty() => {
                            out.push(item(Role::Assistant, text(&b["text"])))
                        }
                        Some("thinking") if !text(&b["thinking"]).trim().is_empty() => {
                            out.push(item(Role::Thinking, clip(text(&b["thinking"]).trim(), THINKING_CHARS)))
                        }
                        Some("tool_use") => {
                            let (name, headline, detail) = tool_use(b);
                            let mut i = item(Role::Tool, headline);
                            i.tool = name;
                            i.detail = detail;
                            out.push(i);
                        }
                        _ => {}
                    }
                }
            }
            "system" => {
                let content = v["content"].as_str().unwrap_or_default();
                match v["subtype"].as_str().unwrap_or_default() {
                    "local_command" => out.extend(slash_command(content).map(|t| item(Role::System, t))),
                    "compact_boundary" => out.push(item(Role::System, "Conversation compacted".into())),
                    // Summaries written for Claude's own use after an absence.
                    "away_summary" => {}
                    _ if !content.is_empty() => out.push(item(Role::System, content.to_string())),
                    _ => {}
                }
            }
            _ => {}
        },
        EventSource::Hook if e.kind == "Notification" => {
            // A permission prompt shows as the pending card and an idle prompt as the
            // session's state; the other notifications (auth, elicitation) are news.
            let routine = matches!(v["notification_type"].as_str(), Some("permission_prompt" | "idle_prompt"));
            if let Some(m) = v["message"].as_str().filter(|_| !routine) {
                out.push(item(Role::System, m.to_string()));
            }
        }
        EventSource::Daemon => match e.kind.as_str() {
            "interrupted" => out.push(item(Role::Daemon, "Interrupted".into())),
            "killed" => out.push(item(Role::Daemon, "Killed".into())),
            _ => {}
        },
        _ => {}
    }
    out
}

/// A user line's text as the person should see it, or nothing. Claude Code writes several
/// things into user lines besides what was typed.
fn user_text(s: &str, meta: bool) -> Option<(Role, String)> {
    const PEER: &str = "Another Claude session sent a message:\n";
    if let Some(rest) = s.strip_prefix(PEER) {
        // A prompt sent over the messaging socket (ADR-0014), followed by a paragraph of
        // advice to the model; the person reads only what they sent.
        let sent = rest
            .split("\n\nThis came from another Claude session")
            .next()
            .unwrap_or(rest)
            .trim_end();
        return Some((Role::User, sent.to_string()));
    }
    if meta {
        // Skill bodies, hook output and other text injected for the model.
        return None;
    }
    let t = s.trim_start();
    if let Some(inner) = tagged(t, "task-notification") {
        let summary = tagged(inner, "summary").map(str::trim).filter(|x| !x.is_empty());
        return Some((
            Role::System,
            match summary {
                Some(x) => format!("Background task: {x}"),
                None => "A background task finished".into(),
            },
        ));
    }
    if t.starts_with("<command-name>") {
        return slash_command(t).map(|c| (Role::System, c));
    }
    if let Some(inner) = tagged(t, "local-command-stdout") {
        let shown = strip_ansi(inner.trim());
        return (!shown.is_empty() && shown != "(no content)")
            .then(|| (Role::ToolResult, clip(&shown, RESULT_CHARS)));
    }
    if t.starts_with("<local-command-caveat>") || t.starts_with("<system-reminder>") {
        return None;
    }
    if let Some(cmd) = tagged(t, "bash-input") {
        return Some((Role::User, format!("! {}", cmd.trim())));
    }
    if t.starts_with("<bash-stdout>") {
        let out = tagged(t, "bash-stdout").unwrap_or_default().trim();
        let err = tagged(t, "bash-stderr").unwrap_or_default().trim();
        let shown = [out, err].iter().filter(|x| !x.is_empty()).copied().collect::<Vec<_>>().join("\n");
        return (!shown.is_empty()).then(|| (Role::ToolResult, clip(&shown, RESULT_CHARS)));
    }
    if t.starts_with("[Request interrupted by user") {
        return Some((Role::System, "Interrupted".into()));
    }
    // Reminders can be appended to what was typed; the person's words stay.
    let own = without_tagged(s, "system-reminder");
    let own = own.trim();
    (!own.is_empty()).then(|| (Role::User, own.to_string()))
}

/// "/compact", "/model haiku": a slash command typed at the terminal.
fn slash_command(s: &str) -> Option<String> {
    let name = tagged(s, "command-name")?.trim();
    let args = tagged(s, "command-args").map(str::trim).unwrap_or_default();
    Some(if args.is_empty() { name.to_string() } else { format!("{name} {args}") })
}

/// The text inside `<tag>…</tag>` when `s` starts with the opening tag.
fn tagged<'a>(s: &'a str, tag: &str) -> Option<&'a str> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = s.find(&open)? + open.len();
    let end = s[start..].find(&close).map(|i| start + i).unwrap_or(s.len());
    Some(&s[start..end])
}

/// `s` with every `<tag>…</tag>` block removed.
fn without_tagged(s: &str, tag: &str) -> String {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find(&open) {
        out.push_str(&rest[..i]);
        rest = match rest[i..].find(&close) {
            Some(j) => &rest[i + j + close.len()..],
            None => "",
        };
    }
    out.push_str(rest);
    out
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                while let Some(&n) = chars.peek() {
                    chars.next();
                    if n.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

/// A tool call as (short name, the one input a person cares about, the rest worth a look).
pub fn tool_use(block: &Value) -> (String, String, Option<String>) {
    let full = block["name"].as_str().unwrap_or("tool");
    // MCP tools are `mcp__<server>__<tool>`; the tool's own name is enough.
    let name = full.rsplit("__").next().unwrap_or(full).to_string();
    let input = &block["input"];
    let s = |k: &str| input[k].as_str().map(str::to_string);
    let (headline, detail) = match full {
        "Bash" => (s("command"), s("description")),
        "Edit" => (
            s("file_path"),
            Some(format!(
                "- {}\n+ {}",
                s("old_string").unwrap_or_default().trim_end(),
                s("new_string").unwrap_or_default().trim_end()
            )),
        ),
        "Write" | "NotebookEdit" => (s("file_path"), s("content").or_else(|| s("new_source"))),
        "Read" => (s("file_path"), None),
        "Glob" | "Grep" => (
            s("pattern").map(|p| match s("path") {
                Some(path) => format!("{p} in {path}"),
                None => p,
            }),
            None,
        ),
        "WebFetch" => (s("url"), s("prompt")),
        "WebSearch" => (s("query"), None),
        "Agent" => (s("description"), s("prompt")),
        "AskUserQuestion" => (input["questions"][0]["question"].as_str().map(str::to_string), None),
        "Skill" => (s("skill"), s("args")),
        "SendMessage" => (s("summary").or_else(|| s("message")), s("message")),
        "TodoWrite" => {
            let todos = input["todos"].as_array().map(|t| t.len()).unwrap_or(0);
            let list = input["todos"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|t| {
                    let mark = match t["status"].as_str() {
                        Some("completed") => "[x]",
                        Some("in_progress") => "[~]",
                        _ => "[ ]",
                    };
                    t["content"].as_str().map(|c| format!("{mark} {c}"))
                })
                .collect::<Vec<_>>()
                .join("\n");
            (Some(format!("{todos} todos")), Some(list))
        }
        _ => {
            // Any other tool: its first string input, the rest as detail.
            let first = ["command", "file_path", "url", "pattern", "query", "description", "prompt", "task_id"]
                .iter()
                .find_map(|k| s(k))
                .or_else(|| {
                    input
                        .as_object()
                        .into_iter()
                        .flatten()
                        .find_map(|(_, v)| v.as_str().map(str::to_string))
                });
            let detail = match input {
                Value::Object(m) if m.len() > 1 => serde_json::to_string_pretty(input).ok(),
                _ => None,
            };
            (first, detail)
        }
    };
    let headline = headline
        .map(|h| clip(h.lines().next().unwrap_or_default().trim(), HEADLINE_CHARS))
        .unwrap_or_default();
    let detail = detail.map(|d| clip(&d, DETAIL_CHARS)).filter(|d| !d.trim().is_empty());
    (name, headline, detail)
}

/// The input a person decides on: a command, a path, a URL, a pattern or a question.
pub fn key_input(input: &Value) -> Option<String> {
    ["command", "file_path", "url", "pattern", "description"]
        .iter()
        .find_map(|k| input[k].as_str())
        .map(str::to_string)
        .or_else(|| {
            input["questions"][0]["question"]
                .as_str()
                .map(str::to_string)
        })
}

fn text(v: &Value) -> String {
    v.as_str().unwrap_or_default().to_string()
}

/// A tool result's text; images and tool definitions are named, not shown.
fn result_text(content: &Value) -> String {
    match content {
        Value::String(s) => s.clone(),
        Value::Array(parts) => parts
            .iter()
            .filter_map(|p| match p["type"].as_str() {
                Some("text") => p["text"].as_str().map(str::to_string),
                Some("image") => Some("[image]".to_string()),
                Some("tool_reference") => p["tool_name"].as_str().map(|n| format!("[tool {n} loaded]")),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

fn clip(s: &str, max: usize) -> String {
    match s.char_indices().nth(max) {
        Some((i, _)) => format!("{}…", &s[..i]),
        None => s.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(source: EventSource, kind: &str, payload: Value) -> Event {
        Event {
            seq: 7,
            time_ms: 1,
            source: source.into(),
            kind: kind.into(),
            payload_json: payload.to_string(),
            ..Default::default()
        }
    }

    fn user(content: Value) -> Event {
        event(EventSource::Transcript, "user", serde_json::json!({"message": {"role": "user", "content": content}}))
    }

    fn shown(e: Event) -> Vec<(Role, String)> {
        items(&e).into_iter().map(|i| (i.role, i.text)).collect()
    }

    #[test]
    fn transcript_lines_become_a_conversation() {
        assert_eq!(shown(user("run the tests".into())), [(Role::User, "run the tests".into())]);
        let assistant = event(
            EventSource::Transcript,
            "assistant",
            serde_json::json!({"message": {"content": [
                {"type": "thinking", "thinking": "hm"},
                {"type": "text", "text": "Running them."},
                {"type": "tool_use", "name": "Bash", "input": {"command": "cargo test", "description": "Run the suite"}}]}}),
        );
        let items = items(&assistant);
        assert_eq!(items[0].role, Role::Thinking);
        assert_eq!(items[1].text, "Running them.");
        assert_eq!((items[2].role.clone(), items[2].tool.as_str(), items[2].text.as_str()), (Role::Tool, "Bash", "cargo test"));
        assert_eq!(items[2].detail.as_deref(), Some("Run the suite"));
        let result = user(serde_json::json!([{"type": "tool_result", "content": "x".repeat(5000), "is_error": true}]));
        let r = super::items(&result);
        assert_eq!(r[0].role, Role::ToolResult);
        assert!(r[0].error);
        assert_eq!(r[0].text.chars().count(), RESULT_CHARS + 1);
    }

    #[test]
    fn tool_calls_name_what_matters() {
        let call = |name: &str, input: Value| tool_use(&serde_json::json!({"name": name, "input": input}));
        assert_eq!(
            call("Edit", serde_json::json!({"file_path": "a.rs", "old_string": "x", "new_string": "y"})),
            ("Edit".into(), "a.rs".into(), Some("- x\n+ y".into()))
        );
        assert_eq!(call("WebSearch", serde_json::json!({"query": "tsnet android", "mode": "standard"})).1, "tsnet android");
        assert_eq!(call("Grep", serde_json::json!({"pattern": "fn main", "path": "src"})).1, "fn main in src");
        let mcp = call("mcp__plugin_x_playwright__browser_navigate", serde_json::json!({"url": "http://h"}));
        assert_eq!((mcp.0.as_str(), mcp.1.as_str()), ("browser_navigate", "http://h"));
        let todo = call("TodoWrite", serde_json::json!({"todos": [{"content": "a", "status": "completed"}, {"content": "b", "status": "pending"}]}));
        assert_eq!((todo.1.as_str(), todo.2.as_deref()), ("2 todos", Some("[x] a\n[ ] b")));
        let unknown = call("Frob", serde_json::json!({"n": 3, "what": "thing"}));
        assert_eq!(unknown.1, "thing");
        assert!(unknown.2.is_some());
    }

    #[test]
    fn what_claude_code_writes_into_user_lines_is_made_readable() {
        let peer = "Another Claude session sent a message:\nReply with just the word osprey.\n\nThis came from another Claude session \u{2014} not typed by your user.";
        assert_eq!(shown(user(peer.into())), [(Role::User, "Reply with just the word osprey.".into())]);
        let note = "<task-notification>\n<task-id>a0</task-id>\n<status>completed</status>\n<summary>Agent \"Verify hooks\" finished</summary>\n<result>…</result>\n</task-notification>";
        assert_eq!(shown(user(note.into())), [(Role::System, "Background task: Agent \"Verify hooks\" finished".into())]);
        let cmd = "<command-name>/model</command-name>\n            <command-message>model</command-message>\n            <command-args>haiku</command-args>";
        assert_eq!(shown(user(cmd.into())), [(Role::System, "/model haiku".into())]);
        assert!(shown(user("<local-command-stdout>(no content)</local-command-stdout>".into())).is_empty());
        assert_eq!(
            shown(user("<local-command-stdout>\u{1b}[2mCompacted\u{1b}[22m</local-command-stdout>".into())),
            [(Role::ToolResult, "Compacted".into())]
        );
        assert!(shown(user("<local-command-caveat>The command below…</local-command-caveat>".into())).is_empty());
        assert_eq!(shown(user("<bash-input>pwd</bash-input>".into())), [(Role::User, "! pwd".into())]);
        assert_eq!(
            shown(user("<bash-stdout>/tmp</bash-stdout><bash-stderr></bash-stderr>".into())),
            [(Role::ToolResult, "/tmp".into())]
        );
        assert_eq!(
            shown(user(serde_json::json!([{"type": "text", "text": "[Request interrupted by user]"}]))),
            [(Role::System, "Interrupted".into())]
        );
        assert_eq!(
            shown(user("fix it\n<system-reminder>be careful</system-reminder>".into())),
            [(Role::User, "fix it".into())]
        );
        let meta = event(
            EventSource::Transcript,
            "user",
            serde_json::json!({"isMeta": true, "message": {"role": "user", "content": [{"type": "text", "text": "Base directory for this skill: /x"}]}}),
        );
        assert!(shown(meta).is_empty());
        let image = user(serde_json::json!([{"type": "tool_result", "content": [{"type": "image", "source": {"data": "AAAA"}}, {"type": "text", "text": "ok"}]}]));
        assert_eq!(shown(image), [(Role::ToolResult, "[image]\nok".into())]);
    }

    #[test]
    fn hooks_and_bookkeeping_show_only_where_they_add_something() {
        let pre = event(EventSource::Hook, "PreToolUse", serde_json::json!({"tool_name": "Bash"}));
        assert!(shown(pre).is_empty());
        let note = event(
            EventSource::Hook,
            "Notification",
            serde_json::json!({"message": "Claude needs your permission to use Bash", "notification_type": "permission_prompt"}),
        );
        assert!(shown(note).is_empty());
        let auth = event(
            EventSource::Hook,
            "Notification",
            serde_json::json!({"message": "Logged in", "notification_type": "auth_success"}),
        );
        assert_eq!(shown(auth)[0].0, Role::System);
        let usage = event(EventSource::Daemon, "usage", serde_json::json!({}));
        assert!(shown(usage).is_empty());
        let interrupted = event(EventSource::Daemon, "interrupted", serde_json::json!({}));
        assert_eq!(shown(interrupted), [(Role::Daemon, "Interrupted".into())]);
        let duration = event(
            EventSource::Transcript,
            "system",
            serde_json::json!({"subtype": "turn_duration", "durationMs": 5}),
        );
        assert!(shown(duration).is_empty());
        let slash = event(
            EventSource::Transcript,
            "system",
            serde_json::json!({"subtype": "local_command", "content": "<command-name>/context</command-name>\n<command-args></command-args>"}),
        );
        assert_eq!(shown(slash), [(Role::System, "/context".into())]);
        let compact = event(EventSource::Transcript, "system", serde_json::json!({"subtype": "compact_boundary", "content": "Conversation compacted"}));
        assert_eq!(shown(compact), [(Role::System, "Conversation compacted".into())]);
        let away = event(EventSource::Transcript, "system", serde_json::json!({"subtype": "away_summary", "content": "Planning…"}));
        assert!(shown(away).is_empty());
    }
}
