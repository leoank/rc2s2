//! `rc2s2 run` against a daemon's wrapper socket, with a shell script in place of Claude
//! Code that records the bytes it receives on its terminal: a key typed at the wrapper
//! reaches the program and releases a request held for its session, an interrupt types
//! Escape, Ctrl-U and the follow-up prompt, and a kill ends the program.

use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use rc2s2_node::outbox::Outbox;
use rc2s2_node::pending::Pending;
use rc2s2_node::registry::Record;
use rc2s2_node::sessions::Sessions;
use rc2s2_node::wrapper::{ToWrapper, Wrappers};

async fn until(what: &str, mut done: impl AsyncFnMut() -> bool) {
    let start = Instant::now();
    while !done().await {
        assert!(start.elapsed() < Duration::from_secs(20), "never {what}");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_wrapper_types_interrupts_reports_keystrokes_and_kills() {
    let dir = tempfile::tempdir().unwrap();
    let (outbox, _rx) = Outbox::new(1024);
    let sessions = Arc::new(Sessions::new(
        dir.path().to_path_buf(),
        outbox.clone(),
        Duration::from_millis(10),
    ));
    let pending = Pending::new(outbox.clone());
    let wrappers = Wrappers::new(sessions.clone(), pending.clone(), outbox);
    let socket = dir.path().join("w/node.sock");
    tokio::spawn(wrappers.clone().serve(Wrappers::bind(&socket).unwrap()));

    let (pid_file, got) = (dir.path().join("pid"), dir.path().join("got"));
    let script = format!(
        "echo $$ > {pid}; stty raw -echo; dd bs=1 count=6 of={got} 2>/dev/null; exec sleep 60",
        pid = pid_file.display(),
        got = got.display()
    );
    let mut wrapper = Command::new(env!("CARGO_BIN_EXE_rc2s2"))
        .args(["run", "--command", "sh", "--wrapper-socket"])
        .arg(&socket)
        .args(["--", "-c", &script])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .unwrap();

    let pid = async || {
        std::fs::read_to_string(&pid_file)
            .ok()
            .and_then(|s| s.trim().parse::<u32>().ok())
    };
    until("the program started", async || pid().await.is_some()).await;
    let record = Record {
        pid: pid().await.unwrap(),
        session_id: "s1".into(),
        cwd: "/w".into(),
        started_at: 1,
        status: "busy".into(),
        messaging_socket_path: String::new(),
    };
    sessions.apply_registry(&[record]).await;
    until("the wrapper said hello", async || {
        sessions.is_wrapped("s1").await
    })
    .await;

    // A person types while a request is held: the key reaches the program and the hold ends.
    let input = serde_json::json!({"session_id": "s1", "tool_name": "Bash", "tool_input": {}});
    let hold = {
        let pending = pending.clone();
        tokio::spawn(async move { pending.hold(&input, Duration::from_secs(30)).await })
    };
    until("the request was held", async || {
        !pending.snapshot().await.is_empty()
    })
    .await;
    let mut stdin = wrapper.stdin.take().unwrap();
    stdin.write_all(b"x").unwrap();
    stdin.flush().unwrap();
    assert_eq!(hold.await.unwrap(), serde_json::json!({}));

    let interrupt = ToWrapper::Interrupt {
        then_prompt: Some("hi".into()),
    };
    wrappers.command("s1", interrupt).await.unwrap();
    until("the program got six bytes", async || {
        std::fs::metadata(&got).is_ok_and(|m| m.len() == 6)
    })
    .await;
    assert_eq!(std::fs::read(&got).unwrap(), b"x\x1b\x15hi\r");

    wrappers.command("s1", ToWrapper::Kill).await.unwrap();
    let start = Instant::now();
    while wrapper.try_wait().unwrap().is_none() {
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "the wrapper outlived the kill"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    until("the wrapper disconnected", async || {
        !sessions.is_wrapped("s1").await
    })
    .await;
    assert!(wrappers.command("s1", ToWrapper::Kill).await.is_err());
}
