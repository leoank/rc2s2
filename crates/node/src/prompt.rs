//! Prompts delivered over a session's cross-session messaging socket (EXP-0005,
//! `spikes/10-messaging-socket/PROTOCOL.md`): newline-delimited JSON on the AF_UNIX socket
//! the registry names, an optional `auth` line with the session's peer token, then one
//! `user` line. The session queues it as a user turn behind the current one. It works for
//! sessions started by hand, so prompting does not need the wrapper.
//!
//! A session that bypasses permissions holds such a message for approval at its terminal.
//! Its receipt goes only to a reply socket registered as a Claude Code session, which the
//! node does not fake, so the node reports the hold from the session's permission mode as
//! the hooks last gave it.

use std::path::Path;
use std::time::Duration;

use serde_json::{Value, json};
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;

use crate::sessions::PromptTarget;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, PartialEq)]
pub enum Delivery {
    /// Queued as a user turn.
    Queued,
    /// Waiting for the person at the terminal to approve it.
    HeldForApproval,
}

pub async fn deliver(
    claude_dir: &Path,
    session_id: &str,
    target: &PromptTarget,
    text: &str,
) -> Result<Delivery, String> {
    if text.trim().is_empty() {
        return Err("the prompt is empty".into());
    }
    let unreachable = |e: std::io::Error| format!("the session's messaging socket: {e}");
    let mut stream = tokio::time::timeout(CONNECT_TIMEOUT, UnixStream::connect(&target.socket))
        .await
        .map_err(|_| "the session's messaging socket did not accept in time".to_string())?
        .map_err(unreachable)?;
    let mut lines = String::new();
    if let Some(token) = peer_token(&claude_dir.join("sessions"), target.pid) {
        lines.push_str(&json!({"type": "auth", "token": token}).to_string());
        lines.push('\n');
    }
    // With the session id, a pid reused by another session drops the message instead of
    // delivering it to the wrong conversation.
    let user = json!({"type": "user", "session_id": session_id,
                      "message": {"role": "user", "content": text}});
    lines.push_str(&user.to_string());
    lines.push('\n');
    stream
        .write_all(lines.as_bytes())
        .await
        .map_err(unreachable)?;
    stream.shutdown().await.map_err(unreachable)?;
    Ok(if target.permission_mode == "bypassPermissions" {
        Delivery::HeldForApproval
    } else {
        Delivery::Queued
    })
}

/// The `peerToken` from `<pid>.<64 hex>.key` beside the registry record. Never logged.
fn peer_token(sessions_dir: &Path, pid: u32) -> Option<String> {
    let prefix = format!("{pid}.");
    let entries = std::fs::read_dir(sessions_dir).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(hex) = name
            .strip_prefix(&prefix)
            .and_then(|rest| rest.strip_suffix(".key"))
        else {
            continue;
        };
        if hex.len() != 64 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            continue;
        }
        let key: Value = serde_json::from_slice(&std::fs::read(entry.path()).ok()?).ok()?;
        return key["peerToken"].as_str().map(str::to_string);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncReadExt;
    use tokio::net::UnixListener;

    fn target(socket: &Path, mode: &str) -> PromptTarget {
        PromptTarget {
            pid: 4242,
            socket: socket.to_path_buf(),
            permission_mode: mode.into(),
        }
    }

    /// Accepts one connection and returns the lines it sent.
    fn listen(socket: &Path) -> tokio::task::JoinHandle<Vec<Value>> {
        let listener = UnixListener::bind(socket).unwrap();
        tokio::spawn(async move {
            let (mut conn, _) = listener.accept().await.unwrap();
            let mut text = String::new();
            conn.read_to_string(&mut text).await.unwrap();
            text.lines()
                .map(|l| serde_json::from_str(l).unwrap())
                .collect()
        })
    }

    #[tokio::test]
    async fn a_prompt_is_one_user_line_after_the_auth_line() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sessions")).unwrap();
        let key = format!("4242.{}.key", "ab".repeat(32));
        std::fs::write(
            dir.path().join("sessions").join(key),
            r#"{"peerToken":"tok","pidDomain":"darwin","procStart":"x"}"#,
        )
        .unwrap();
        // A key of another pid must not be used.
        std::fs::write(
            dir.path()
                .join("sessions")
                .join(format!("42.{}.key", "cd".repeat(32))),
            r#"{"peerToken":"other"}"#,
        )
        .unwrap();
        let socket = dir.path().join("s.sock");
        let got = listen(&socket);
        let delivery = deliver(
            dir.path(),
            "s1",
            &target(&socket, "default"),
            "run the tests",
        )
        .await
        .unwrap();
        assert_eq!(delivery, Delivery::Queued);
        assert_eq!(
            got.await.unwrap(),
            [
                json!({"type": "auth", "token": "tok"}),
                json!({"type": "user", "session_id": "s1",
                       "message": {"role": "user", "content": "run the tests"}})
            ]
        );
    }

    #[tokio::test]
    async fn without_a_key_it_sends_no_auth_and_reports_a_bypass_hold() {
        let dir = tempfile::tempdir().unwrap();
        let socket = dir.path().join("s.sock");
        let got = listen(&socket);
        let delivery = deliver(
            dir.path(),
            "s1",
            &target(&socket, "bypassPermissions"),
            "hi",
        )
        .await
        .unwrap();
        assert_eq!(delivery, Delivery::HeldForApproval);
        assert_eq!(got.await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn an_unreachable_socket_or_an_empty_prompt_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let t = target(&dir.path().join("gone.sock"), "default");
        assert!(deliver(dir.path(), "s1", &t, "hi").await.is_err());
        assert!(deliver(dir.path(), "s1", &t, "  ").await.is_err());
    }
}
