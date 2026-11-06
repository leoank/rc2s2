//! Phase 2 step 1 end to end on loopback (ADR-0034): a client spawns a session on a node
//! allowed to, with a stand-in for claude; the session is listed as spawned under the id
//! the spawn returned, takes a prompt and a kill over its input and ends; spawning outside
//! the node's roots and on a node without roots is refused; keep-awake is switched.

mod common;

use std::os::unix::fs::PermissionsExt;
use std::time::Duration;

use common::{HubProc, start_node, start_spawning_node};
use rc2s2_proto::control::v1::control_service_client::ControlServiceClient;
use rc2s2_proto::control::v1::*;
use rc2s2_proto::event::v1::{Launch, Session};
use tonic::transport::Channel;

const HOLD: Duration = Duration::from_secs(30);

/// Logs every input line and answers each `user` line with a `result`; exits on EOF.
const FAKE: &str = r#"#!/bin/sh
while IFS= read -r line; do
  echo "$line" >> LOG
  case "$line" in
    *'"type":"user"'*) echo '{"type":"result","subtype":"success"}' ;;
  esac
done
"#;

fn req<T>(body: T) -> tonic::Request<T> {
    let mut r = tonic::Request::new(body);
    r.metadata_mut()
        .insert("authorization", "Bearer t-client".parse().unwrap());
    r
}

async fn connected(c: &mut ControlServiceClient<Channel>) -> Node {
    loop {
        let nodes = c
            .list_nodes(req(ListNodesRequest {}))
            .await
            .unwrap()
            .into_inner()
            .nodes;
        if let Some(n) = nodes.into_iter().find(|n| n.connected) {
            return n;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

async fn session(c: &mut ControlServiceClient<Channel>, id: &str, ended: bool) -> Session {
    for _ in 0..250 {
        let all = c
            .list_sessions(req(ListSessionsRequest {
                node_id: String::new(),
                include_ended: true,
            }))
            .await
            .unwrap()
            .into_inner()
            .sessions;
        if let Some(s) = all.into_iter().find(|s| s.id == id && s.ended_ms.is_some() == ended) {
            return s;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("session {id} never showed with ended={ended}");
}

async fn log_has(log: &std::path::Path, text: &str) {
    for _ in 0..250 {
        if std::fs::read_to_string(log).unwrap_or_default().contains(text) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("{text:?} never reached the fake claude");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_client_spawns_prompts_and_kills_a_headless_session() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("work");
    std::fs::create_dir_all(root.join("repo")).unwrap();
    let log = dir.path().join("input.log");
    let fake = dir.path().join("claude");
    std::fs::write(&fake, FAKE.replace("LOG", &format!("'{}'", log.display()))).unwrap();
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();

    let (hub, addr) = HubProc::start(&dir.path().join("hub.db"), "127.0.0.1:0");
    let _node = start_spawning_node(
        &addr,
        "t-node",
        &dir.path().join(".claude"),
        HOLD,
        &fake.to_string_lossy(),
        std::slice::from_ref(&root),
    )
    .await;
    let mut c = ControlServiceClient::connect(format!("http://{addr}"))
        .await
        .unwrap();
    let node = connected(&mut c).await;
    let root = root.canonicalize().unwrap();
    assert_eq!(node.spawn_roots, vec![root.to_string_lossy().into_owned()]);

    let cwd = root.join("repo").to_string_lossy().into_owned();
    let spawn = SpawnRequest {
        node_id: node.id.clone(),
        cwd: cwd.clone(),
        prompt: "write the tests".into(),
        model: String::new(),
        create: false,
    };
    let r = c.spawn(req(spawn)).await.unwrap().into_inner();
    assert!(r.ok, "{}", r.error);
    assert!(uuid::Uuid::parse_str(&r.session_id).is_ok(), "{}", r.session_id);
    let id = r.session_id;
    log_has(&log, "write the tests").await;
    let s = session(&mut c, &id, false).await;
    assert_eq!(s.launch(), Launch::Spawned);
    assert_eq!(s.cwd, cwd);

    let prompt = PromptRequest {
        node_id: node.id.clone(),
        session_id: id.clone(),
        text: "and run them".into(),
    };
    let r = c.prompt(req(prompt)).await.unwrap().into_inner();
    assert!(r.ok && r.note.is_empty(), "{} {}", r.error, r.note);
    log_has(&log, "and run them").await;

    let kill = KillRequest {
        node_id: node.id.clone(),
        session_id: id.clone(),
    };
    let r = c.kill(req(kill)).await.unwrap().into_inner();
    assert!(r.ok, "{}", r.error);
    session(&mut c, &id, true).await;

    // Outside the roots: refused, with the reason.
    let outside = SpawnRequest {
        node_id: node.id.clone(),
        cwd: dir.path().to_string_lossy().into_owned(),
        prompt: "hi".into(),
        model: String::new(),
        create: false,
    };
    let r = c.spawn(req(outside)).await.unwrap().into_inner();
    assert!(!r.ok && r.error.contains("spawn root"), "{}", r.error);
    assert!(r.session_id.is_empty());

    let on = SetKeepAwakeRequest {
        node_id: node.id.clone(),
        on: true,
    };
    let r = c.set_keep_awake(req(on)).await.unwrap().into_inner();
    assert!(r.ok, "{}", r.error);
    hub.stop();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_node_without_spawn_roots_refuses_to_spawn() {
    let dir = tempfile::tempdir().unwrap();
    let (hub, addr) = HubProc::start(&dir.path().join("hub.db"), "127.0.0.1:0");
    let _node = start_node(&addr, "t-node", dir.path(), HOLD).await;
    let mut c = ControlServiceClient::connect(format!("http://{addr}"))
        .await
        .unwrap();
    let node = connected(&mut c).await;
    assert!(node.spawn_roots.is_empty());
    let spawn = SpawnRequest {
        node_id: node.id,
        cwd: dir.path().to_string_lossy().into_owned(),
        prompt: "hi".into(),
        model: String::new(),
        create: false,
    };
    let r = c.spawn(req(spawn)).await.unwrap().into_inner();
    assert!(!r.ok && r.error.contains("--spawn-root"), "{}", r.error);
    hub.stop();
}
