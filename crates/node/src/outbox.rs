//! Everything the node tells the hub goes through the outbox as a `NodeMessage`. Events get
//! their per-node sequence number and time here, so the order the hub sees is the order the
//! node observed.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use rc2s2_proto::event::v1::{Event, EventSource, HealthSample, Session};
use rc2s2_proto::node::v1::{NodeMessage, node_message::Body};
use tokio::sync::{Mutex, mpsc};

#[derive(Clone)]
pub struct Outbox {
    tx: mpsc::Sender<NodeMessage>,
    // Held from numbering to sending, so events enter the channel in seq order.
    seq: Arc<Mutex<u64>>,
}

/// Where numbering starts: the time in microseconds, so a restarted node continues above
/// every seq it sent before (unless the clock went back or it sent over a million events a
/// second), and the hub's "already stored" check by seq stays correct across restarts.
fn first_seq() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_micros() as u64)
        .unwrap_or(0)
}

impl Outbox {
    pub fn new(capacity: usize) -> (Self, mpsc::Receiver<NodeMessage>) {
        let (tx, rx) = mpsc::channel(capacity);
        (
            Self {
                tx,
                seq: Arc::new(Mutex::new(first_seq())),
            },
            rx,
        )
    }

    pub async fn event(
        &self,
        session_id: &str,
        source: EventSource,
        kind: &str,
        payload_json: String,
    ) {
        self.line(session_id, source, kind, payload_json, String::new())
            .await;
    }

    /// An event the hub stores once per session and `line_id`, however often it is sent.
    pub async fn line(
        &self,
        session_id: &str,
        source: EventSource,
        kind: &str,
        payload_json: String,
        line_id: String,
    ) {
        let mut seq = self.seq.lock().await;
        *seq += 1;
        let event = Event {
            session_id: session_id.to_string(),
            seq: *seq,
            time_ms: now_ms(),
            source: source.into(),
            kind: kind.to_string(),
            payload_json,
            line_id,
        };
        self.send(Body::Event(event)).await;
    }

    pub async fn session(&self, session: Session) {
        self.send(Body::Session(session)).await;
    }

    pub async fn health(&self, sample: HealthSample) {
        self.send(Body::Health(sample)).await;
    }

    async fn send(&self, body: Body) {
        // The receiver only goes away when the node is shutting down.
        let _ = self.tx.send(NodeMessage { body: Some(body) }).await;
    }
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
