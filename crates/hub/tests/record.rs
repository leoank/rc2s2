//! A real node link against a real hub on loopback: events are recorded exactly once across
//! a hub restart (the node buffers and resends) and a node restart (seq keeps increasing,
//! re-read transcript lines are recognised by their id), sessions arrive with the
//! registration, and an unknown token is refused.

mod common;

use std::time::Duration;

use common::{HubProc, start_node};
use rc2s2_hub::db::Db;
use rc2s2_node::outbox::Outbox;
use rc2s2_proto::event::v1::EventSource;

const HOLD: Duration = Duration::from_secs(5);

async fn emit(outbox: &Outbox, n: usize) {
    for i in 0..n {
        outbox
            .event(
                "s1",
                EventSource::Hook,
                "PreToolUse",
                format!(r#"{{"i":{i}}}"#),
            )
            .await;
    }
}

/// Transcript lines `ids` of session `session`, as the tailer sends them.
async fn emit_lines(outbox: &Outbox, session: &str, ids: impl Iterator<Item = usize>) {
    for i in ids {
        outbox
            .line(
                session,
                EventSource::Transcript,
                "assistant",
                format!(r#"{{"uuid":"u{i}"}}"#),
                format!("u{i}"),
            )
            .await;
    }
}

async fn wait_for(db: &Db, count: usize) -> Vec<u64> {
    for _ in 0..200 {
        let seqs = db.event_seqs("test-node").await.unwrap();
        if seqs.len() >= count {
            return seqs;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("the hub never stored {count} events");
}

#[tokio::test(flavor = "multi_thread")]
async fn events_are_recorded_once_across_hub_and_node_restarts() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("hub.db");
    let (hub, addr) = HubProc::start(&db_path, "127.0.0.1:0");
    let db = Db::open(&db_path).await.unwrap();

    let n = start_node(&addr, "t-node", dir.path(), HOLD).await;
    let (outbox, sessions, node) = (n.outbox, n.sessions, n.task);
    let start =
        serde_json::json!({"hook_event_name": "SessionStart", "session_id": "s1", "cwd": "/w"});
    sessions.apply_hook(&start).await;
    emit(&outbox, 50).await;
    assert_eq!(wait_for(&db, 50).await.len(), 50);

    // The hub goes away; the node keeps its events and resends them when the hub is back.
    hub.stop();
    emit(&outbox, 30).await;
    let (hub, _) = HubProc::start(&db_path, &addr);
    let seqs = wait_for(&db, 80).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(
        db.event_seqs("test-node").await.unwrap().len(),
        80,
        "an event was stored twice"
    );
    assert!(seqs.windows(2).all(|w| w[0] < w[1]));
    assert_eq!(db.session_ids("test-node").await.unwrap(), ["s1"]);

    // The node restarts with a fresh outbox; its numbering continues above what it sent.
    node.abort();
    let _ = node.await;
    drop(outbox);
    let n = start_node(&addr, "t-node", dir.path(), HOLD).await;
    let (outbox, _sessions, _node) = (n.outbox, n.sessions, n.task);
    emit(&outbox, 20).await;
    let seqs = wait_for(&db, 100).await;
    assert_eq!(seqs.len(), 100);
    hub.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_restarted_node_re_reading_its_transcripts_adds_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("hub.db");
    let (hub, addr) = HubProc::start(&db_path, "127.0.0.1:0");
    let db = Db::open(&db_path).await.unwrap();

    let n = start_node(&addr, "t-node", dir.path(), HOLD).await;
    let (outbox, _sessions, node) = (n.outbox, n.sessions, n.task);
    emit_lines(&outbox, "s1", 0..10).await;
    // A forked session repeats a line of its parent; it belongs to both.
    emit_lines(&outbox, "s2", 0..1).await;
    assert_eq!(wait_for(&db, 11).await.len(), 11);

    // The restarted node reads s1 from the start under new seqs, then one new line.
    node.abort();
    let _ = node.await;
    drop(outbox);
    let n = start_node(&addr, "t-node", dir.path(), HOLD).await;
    let (outbox, _sessions, _node) = (n.outbox, n.sessions, n.task);
    emit_lines(&outbox, "s1", 0..11).await;
    assert_eq!(wait_for(&db, 12).await.len(), 12);
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(db.event_seqs("test-node").await.unwrap().len(), 12);
    hub.stop();
}

#[tokio::test]
async fn an_unknown_token_is_refused_and_records_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("hub.db");
    let (hub, addr) = HubProc::start(&db_path, "127.0.0.1:0");
    let db = Db::open(&db_path).await.unwrap();
    let n = start_node(&addr, "wrong", dir.path(), HOLD).await;
    let (outbox, _sessions, _node) = (n.outbox, n.sessions, n.task);
    emit(&outbox, 5).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(db.event_seqs("test-node").await.unwrap().is_empty());
    hub.stop();
}
