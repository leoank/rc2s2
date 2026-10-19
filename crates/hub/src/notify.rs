//! Push to the phone through ntfy (ADR-0008): a held permission request or question, and a
//! turn that finished, so the person knows a session is waiting. One POST per message to a
//! topic URL, `--ntfy http(s)://host/topic`. The PermissionRequest itself is the trigger,
//! since Claude Code's own Notification hook arrives only after it returns (EXP-0001).
//! Sending never blocks the node's stream: each message goes out on its own task and a
//! failure is only logged.

use rc2s2_proto::event::v1::{Event, EventSource, PendingKind, PendingRequest};
use serde_json::Value;

/// How much of a command or question goes into a notification.
const BODY_CHARS: usize = 200;

pub struct Notifier {
    client: reqwest::Client,
    topic_url: String,
}

/// One notification: ntfy's title, body and priority (1 to 5).
#[derive(Debug, PartialEq)]
pub struct Message {
    pub title: String,
    pub body: String,
    pub priority: u8,
}

impl Notifier {
    pub fn new(topic_url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            topic_url,
        }
    }

    pub fn pending(&self, node: &str, p: &PendingRequest) {
        self.send(for_pending(node, p));
    }

    pub fn event(&self, node: &str, e: &Event) {
        if let Some(m) = for_event(node, e) {
            self.send(m);
        }
    }

    fn send(&self, m: Message) {
        let request = self
            .client
            .post(&self.topic_url)
            .header("Title", m.title)
            .header("Priority", m.priority.to_string())
            .body(m.body);
        tokio::spawn(async move {
            match request.send().await.and_then(|r| r.error_for_status()) {
                Ok(_) => {}
                Err(e) => tracing::warn!(error = %e, "ntfy publish failed"),
            }
        });
    }
}

pub fn for_pending(node: &str, p: &PendingRequest) -> Message {
    let input: Value = serde_json::from_str(&p.tool_input_json).unwrap_or_default();
    if p.kind() == PendingKind::Question {
        let question = input["questions"][0]["question"]
            .as_str()
            .unwrap_or("a question");
        return Message {
            title: format!("{node}: Claude asks"),
            body: clip(question),
            priority: 4,
        };
    }
    // The part of the input a person decides on: the command, or the file it touches.
    let what = ["command", "file_path", "url", "pattern"]
        .iter()
        .find_map(|k| input[k].as_str())
        .map(str::to_string)
        .unwrap_or_else(|| p.tool_input_json.clone());
    Message {
        title: format!("{node}: allow {}?", p.tool_name),
        body: clip(&what),
        priority: 4,
    }
}

/// A finished turn: the Stop hook. Other events do not notify.
pub fn for_event(node: &str, e: &Event) -> Option<Message> {
    if e.source() != EventSource::Hook || e.kind != "Stop" {
        return None;
    }
    let input: Value = serde_json::from_str(&e.payload_json).unwrap_or_default();
    Some(Message {
        title: format!("{node}: turn finished"),
        body: clip(input["cwd"].as_str().unwrap_or(&e.session_id)),
        priority: 3,
    })
}

fn clip(s: &str) -> String {
    match s.char_indices().nth(BODY_CHARS) {
        Some((i, _)) => format!("{}…", &s[..i]),
        None => s.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pending(kind: PendingKind, tool: &str, input: &str) -> PendingRequest {
        PendingRequest {
            kind: kind.into(),
            tool_name: tool.into(),
            tool_input_json: input.into(),
            ..Default::default()
        }
    }

    #[test]
    fn messages_say_what_waits_and_where() {
        let p = pending(
            PendingKind::Permission,
            "Bash",
            r#"{"command":"make deploy"}"#,
        );
        assert_eq!(
            for_pending("mac", &p),
            Message {
                title: "mac: allow Bash?".into(),
                body: "make deploy".into(),
                priority: 4
            }
        );
        let q = pending(
            PendingKind::Question,
            "AskUserQuestion",
            r#"{"questions":[{"question":"Ship it?"}]}"#,
        );
        assert_eq!(for_pending("mac", &q).body, "Ship it?");
        let long = format!(r#"{{"command":"{}"}}"#, "x".repeat(500));
        let clipped = for_pending("mac", &pending(PendingKind::Permission, "Bash", &long)).body;
        assert_eq!(clipped.chars().count(), BODY_CHARS + 1);

        let stop = Event {
            source: EventSource::Hook.into(),
            kind: "Stop".into(),
            payload_json: r#"{"cwd":"/w/rc2s2"}"#.into(),
            ..Default::default()
        };
        assert_eq!(for_event("mac", &stop).unwrap().body, "/w/rc2s2");
        let other = Event {
            kind: "PreToolUse".into(),
            ..stop
        };
        assert!(for_event("mac", &other).is_none());
    }
}
