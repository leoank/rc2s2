//! The node's view of its sessions, merged from two sources: hooks, which name the session
//! and its transcript, and the session registry, which adds the pid and idle or busy status
//! and finds sessions that predate the node. Each known session has a transcript tailer.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rc2s2_proto::event::v1::{Launch, Session, SessionStatus};
use serde_json::Value;
use tokio::sync::{Mutex, watch};
use tokio::task::JoinHandle;

use crate::outbox::{Outbox, now_ms};
use crate::registry::Record;
use crate::transcript::{self, TailState};

const HARNESS: &str = "claude-code";

pub struct Sessions {
    claude_dir: PathBuf,
    outbox: Outbox,
    poll: Duration,
    entries: Mutex<HashMap<String, Entry>>,
}

struct Entry {
    session: Session,
    /// Seen in the registry, so leaving it means the process is gone.
    in_registry: bool,
    tail: Option<Tail>,
}

struct Tail {
    stop: watch::Sender<bool>,
    task: JoinHandle<TailState>,
}

impl Sessions {
    pub fn new(claude_dir: PathBuf, outbox: Outbox, poll: Duration) -> Self {
        Self {
            claude_dir,
            outbox,
            poll,
            entries: Mutex::new(HashMap::new()),
        }
    }

    /// Every session the node knows, for the registration message after a (re)connect.
    pub async fn snapshot(&self) -> Vec<Session> {
        self.entries
            .lock()
            .await
            .values()
            .map(|e| e.session.clone())
            .collect()
    }

    /// Updates the session a hook input names. SessionEnd ends it; any other hook on an
    /// ended session reopens it, as `--resume` does.
    pub async fn apply_hook(&self, input: &Value) {
        let Some(id) = input["session_id"].as_str() else {
            return;
        };
        let mut entries = self.entries.lock().await;
        let entry = entries
            .entry(id.to_string())
            .or_insert_with(|| Entry::new(id));
        let before = entry.session.clone();
        let s = &mut entry.session;
        if let Some(cwd) = input["cwd"].as_str() {
            s.cwd = cwd.to_string();
        }
        if let Some(path) = input["transcript_path"].as_str() {
            s.transcript_path = path.to_string();
        }
        if input["hook_event_name"] == "SessionEnd" {
            self.end(entry).await;
        } else {
            entry.session.ended_ms = None;
            self.ensure_tail(entry).await;
        }
        if entry.session != before {
            self.outbox.session(entry.session.clone()).await;
        }
    }

    /// Applies a registry scan: records update their sessions, and a session that was in the
    /// registry and has left it has ended.
    pub async fn apply_registry(&self, live: &[Record]) {
        let mut entries = self.entries.lock().await;
        for record in live {
            let entry = entries
                .entry(record.session_id.clone())
                .or_insert_with(|| Entry::new(&record.session_id));
            let before = entry.session.clone();
            entry.in_registry = true;
            let s = &mut entry.session;
            s.pid = record.pid;
            s.status = status(&record.status).into();
            if s.cwd.is_empty() {
                s.cwd = record.cwd.clone();
            }
            if s.transcript_path.is_empty() {
                s.transcript_path =
                    transcript_path(&self.claude_dir, &record.cwd, &record.session_id)
                        .to_string_lossy()
                        .into_owned();
            }
            if record.started_at > 0 {
                s.started_ms = record.started_at;
            }
            s.ended_ms = None;
            self.ensure_tail(entry).await;
            if entry.session != before {
                self.outbox.session(entry.session.clone()).await;
            }
        }
        for entry in entries.values_mut() {
            let gone = entry.in_registry && !live.iter().any(|r| r.session_id == entry.session.id);
            if gone && entry.session.ended_ms.is_none() {
                self.end(entry).await;
                self.outbox.session(entry.session.clone()).await;
            }
        }
    }

    async fn ensure_tail(&self, entry: &mut Entry) {
        if entry.session.transcript_path.is_empty() {
            return;
        }
        let state = match entry.tail.take() {
            Some(tail) if !*tail.stop.borrow() => {
                entry.tail = Some(tail);
                return;
            }
            Some(tail) => tail.task.await.unwrap_or_default(),
            None => TailState::default(),
        };
        let (stop, stop_rx) = watch::channel(false);
        let task = tokio::spawn(transcript::tail(
            entry.session.id.clone(),
            PathBuf::from(&entry.session.transcript_path),
            self.outbox.clone(),
            self.poll,
            stop_rx,
            state,
        ));
        entry.tail = Some(Tail { stop, task });
    }

    async fn end(&self, entry: &mut Entry) {
        entry.session.ended_ms.get_or_insert_with(now_ms);
        entry.session.status = SessionStatus::Unspecified.into();
        entry.in_registry = false;
        if let Some(tail) = &entry.tail {
            // The tailer reads once more before it stops, so the last lines are not lost.
            let _ = tail.stop.send(true);
        }
    }
}

impl Entry {
    fn new(id: &str) -> Self {
        let session = Session {
            id: id.to_string(),
            harness: HARNESS.to_string(),
            launch: Launch::Bare.into(),
            started_ms: now_ms(),
            ..Default::default()
        };
        Self {
            session,
            in_registry: false,
            tail: None,
        }
    }
}

fn status(registry_status: &str) -> SessionStatus {
    match registry_status {
        "idle" => SessionStatus::Idle,
        "busy" => SessionStatus::Busy,
        _ => SessionStatus::Unspecified,
    }
}

/// `<claude dir>/projects/<cwd with every non-alphanumeric replaced by "-">/<session>.jsonl`
/// (EXP-0002), for sessions found in the registry before any hook named the path.
pub fn transcript_path(claude_dir: &Path, cwd: &str, session_id: &str) -> PathBuf {
    let dir: String = cwd
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    claude_dir
        .join("projects")
        .join(dir)
        .join(format!("{session_id}.jsonl"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rc2s2_proto::node::v1::node_message::Body;
    use tokio::sync::mpsc;

    fn drain_sessions(rx: &mut mpsc::Receiver<rc2s2_proto::node::v1::NodeMessage>) -> Vec<Session> {
        let mut out = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            if let Some(Body::Session(s)) = msg.body {
                out.push(s);
            }
        }
        out
    }

    #[test]
    fn transcript_path_encodes_the_cwd() {
        let p = transcript_path(Path::new("/h/.claude"), "/Users/ank/work.dir", "abc");
        assert_eq!(
            p,
            Path::new("/h/.claude/projects/-Users-ank-work-dir/abc.jsonl")
        );
    }

    #[tokio::test]
    async fn hooks_and_registry_merge_into_one_session() {
        let dir = tempfile::tempdir().unwrap();
        let (outbox, mut rx) = Outbox::new(64);
        let sessions = Sessions::new(dir.path().to_path_buf(), outbox, Duration::from_millis(10));

        let start = serde_json::json!({"hook_event_name": "SessionStart", "session_id": "s1",
            "cwd": "/w", "transcript_path": dir.path().join("s1.jsonl")});
        sessions.apply_hook(&start).await;
        let record = Record {
            pid: 42,
            session_id: "s1".into(),
            cwd: "/w".into(),
            started_at: 7,
            status: "busy".into(),
        };
        sessions.apply_registry(std::slice::from_ref(&record)).await;
        // Unchanged scans say nothing.
        sessions.apply_registry(&[record]).await;
        // The process left the registry: the session ended.
        sessions.apply_registry(&[]).await;

        let seen = drain_sessions(&mut rx);
        assert_eq!(seen.len(), 3);
        assert_eq!(seen[0].cwd, "/w");
        assert_eq!((seen[1].pid, seen[1].status()), (42, SessionStatus::Busy));
        assert!(seen[2].ended_ms.is_some());
    }

    #[tokio::test]
    async fn a_hook_after_session_end_reopens_the_session() {
        let dir = tempfile::tempdir().unwrap();
        let (outbox, mut rx) = Outbox::new(64);
        let sessions = Sessions::new(dir.path().to_path_buf(), outbox, Duration::from_millis(10));
        let hook = |name: &str| serde_json::json!({"hook_event_name": name, "session_id": "s1", "cwd": "/w"});

        sessions.apply_hook(&hook("SessionStart")).await;
        sessions.apply_hook(&hook("SessionEnd")).await;
        sessions.apply_hook(&hook("UserPromptSubmit")).await;

        let seen = drain_sessions(&mut rx);
        assert_eq!(seen.len(), 3);
        assert!(seen[1].ended_ms.is_some());
        assert!(seen[2].ended_ms.is_none());
    }
}
