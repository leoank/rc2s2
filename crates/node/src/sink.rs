//! Until the hub link exists, the outbox drains to stdout as JSON lines: one object per
//! message, with the harness's own JSON embedded rather than quoted.

use rc2s2_proto::event::v1::{Event, HealthSample, Session};
use rc2s2_proto::node::v1::{NodeMessage, node_message::Body};
use serde_json::{Map, Value, json};
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;

pub async fn stdout(mut rx: mpsc::Receiver<NodeMessage>) {
    let mut out = tokio::io::stdout();
    while let Some(msg) = rx.recv().await {
        let Some(line) = msg.body.as_ref().map(to_json) else {
            continue;
        };
        let mut bytes = line.to_string().into_bytes();
        bytes.push(b'\n');
        if out.write_all(&bytes).await.is_err() || out.flush().await.is_err() {
            return;
        }
    }
}

pub fn to_json(body: &Body) -> Value {
    match body {
        Body::Event(e) => event(e),
        Body::Session(s) => session(s),
        Body::Health(h) => health(h),
        other => json!({ "type": "other", "debug": format!("{other:?}") }),
    }
}

fn event(e: &Event) -> Value {
    let payload = serde_json::from_str(&e.payload_json)
        .unwrap_or_else(|_| Value::String(e.payload_json.clone()));
    json!({
        "type": "event", "seq": e.seq, "time_ms": e.time_ms, "session_id": e.session_id,
        "source": e.source().as_str_name(), "kind": e.kind, "payload": payload,
    })
}

fn session(s: &Session) -> Value {
    json!({
        "type": "session", "id": s.id, "harness": s.harness, "cwd": s.cwd,
        "transcript_path": s.transcript_path, "launch": s.launch().as_str_name(), "pid": s.pid,
        "status": s.status().as_str_name(), "started_ms": s.started_ms, "ended_ms": s.ended_ms,
    })
}

fn health(h: &HealthSample) -> Value {
    let metrics: Map<String, Value> = h
        .metrics
        .iter()
        .map(|m| (m.name.clone(), json!(m.value)))
        .collect();
    json!({ "type": "health", "time_ms": h.time_ms, "metrics": metrics })
}
