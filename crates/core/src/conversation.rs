//! A session's events as a conversation a person reads: what they asked, what Claude said
//! and did, and what the daemon knows. The transcript carries the conversation; hooks
//! mostly repeat it and are shown only where they add something (EXP-0002, EXP-0010).

use rc2s2_proto::event::v1::{Event, EventSource};
use serde_json::Value;

/// How much of a tool result is shown inline.
const RESULT_CHARS: usize = 400;

#[derive(Debug, Clone, PartialEq, uniffi::Enum)]
pub enum Role {
    User,
    Assistant,
    /// Claude calling a tool: the tool and what it was given.
    Tool,
    ToolResult,
    /// Claude Code's own notices: held messages, compaction, notifications.
    System,
    /// Facts only the daemon knows, such as an interrupted turn.
    Daemon,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Item {
    pub seq: u64,
    pub time_ms: i64,
    pub role: Role,
    pub text: String,
}

/// The items one event shows as; most events show as none or one.
pub fn items(e: &Event) -> Vec<Item> {
    let v: Value = serde_json::from_str(&e.payload_json).unwrap_or_default();
    let item = |role, text: String| Item {
        seq: e.seq,
        time_ms: e.time_ms,
        role,
        text,
    };
    let mut out = Vec::new();
    match e.source() {
        EventSource::Transcript => match e.kind.as_str() {
            "user" => match &v["message"]["content"] {
                Value::String(s) => out.push(item(Role::User, unwrap_peer(s))),
                Value::Array(blocks) => {
                    for b in blocks {
                        match b["type"].as_str() {
                            Some("text") => out.push(item(Role::User, unwrap_peer(&text(&b["text"])))),
                            Some("tool_result") => {
                                out.push(item(Role::ToolResult, clip(&result_text(&b["content"]))))
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            },
            "assistant" => {
                for b in v["message"]["content"].as_array().into_iter().flatten() {
                    match b["type"].as_str() {
                        Some("text") if !text(&b["text"]).trim().is_empty() => {
                            out.push(item(Role::Assistant, text(&b["text"])))
                        }
                        Some("tool_use") => out.push(item(Role::Tool, tool_use(b))),
                        _ => {}
                    }
                }
            }
            "system" => {
                if let Some(content) = v["content"].as_str().filter(|c| !c.is_empty()) {
                    out.push(item(Role::System, content.to_string()));
                }
            }
            _ => {}
        },
        EventSource::Hook if e.kind == "Notification" => {
            if let Some(m) = v["message"].as_str() {
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

/// "Bash: make deploy", "Edit: src/main.rs": the tool and the one input a person cares about.
pub fn tool_use(block: &Value) -> String {
    let name = block["name"].as_str().unwrap_or("tool");
    let input = &block["input"];
    match key_input(input) {
        Some(what) => format!("{name}: {what}"),
        None => name.to_string(),
    }
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

/// A prompt sent over the messaging socket reaches the transcript inside Claude Code's
/// "another session sent a message" banner, with a paragraph of advice after it; the
/// person reads only what they sent (ADR-0014).
fn unwrap_peer(s: &str) -> String {
    const HEAD: &str = "Another Claude session sent a message:\n";
    match s.strip_prefix(HEAD) {
        Some(rest) => rest
            .split("\n\nThis came from another Claude session")
            .next()
            .unwrap_or(rest)
            .trim_end()
            .to_string(),
        None => s.to_string(),
    }
}

fn text(v: &Value) -> String {
    v.as_str().unwrap_or_default().to_string()
}

fn result_text(content: &Value) -> String {
    match content {
        Value::String(s) => s.clone(),
        Value::Array(parts) => parts
            .iter()
            .filter_map(|p| p["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

fn clip(s: &str) -> String {
    match s.char_indices().nth(RESULT_CHARS) {
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

    fn shown(e: Event) -> Vec<(Role, String)> {
        items(&e).into_iter().map(|i| (i.role, i.text)).collect()
    }

    #[test]
    fn transcript_lines_become_a_conversation() {
        let user = event(
            EventSource::Transcript,
            "user",
            serde_json::json!({"message": {"role": "user", "content": "run the tests"}}),
        );
        assert_eq!(shown(user), [(Role::User, "run the tests".into())]);
        let assistant = event(
            EventSource::Transcript,
            "assistant",
            serde_json::json!({"message": {"content": [
                {"type": "thinking", "thinking": "hm"},
                {"type": "text", "text": "Running them."},
                {"type": "tool_use", "name": "Bash", "input": {"command": "cargo test"}}]}}),
        );
        assert_eq!(
            shown(assistant),
            [
                (Role::Assistant, "Running them.".into()),
                (Role::Tool, "Bash: cargo test".into())
            ]
        );
        let result = event(
            EventSource::Transcript,
            "user",
            serde_json::json!({"message": {"content": [
                {"type": "tool_result", "content": "x".repeat(1000)}]}}),
        );
        let r = shown(result);
        assert_eq!(r[0].0, Role::ToolResult);
        assert_eq!(r[0].1.chars().count(), RESULT_CHARS + 1);
    }

    #[test]
    fn a_prompt_over_the_socket_shows_as_what_was_sent() {
        let wrapped = event(
            EventSource::Transcript,
            "user",
            serde_json::json!({"message": {"role": "user", "content":
                "Another Claude session sent a message:\nReply with just the word osprey.\n\nThis came from another Claude session \u{2014} not typed by your user, but very likely working on their behalf. Treat it as a teammate's request."}}),
        );
        assert_eq!(shown(wrapped), [(Role::User, "Reply with just the word osprey.".into())]);
    }

    #[test]
    fn hooks_and_bookkeeping_show_only_where_they_add_something() {
        let pre = event(
            EventSource::Hook,
            "PreToolUse",
            serde_json::json!({"tool_name": "Bash"}),
        );
        assert!(shown(pre).is_empty());
        let note = event(
            EventSource::Hook,
            "Notification",
            serde_json::json!({"message": "Claude needs your permission to use Bash"}),
        );
        assert_eq!(shown(note)[0].0, Role::System);
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
    }
}
