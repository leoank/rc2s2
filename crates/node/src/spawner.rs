//! Headless sessions the node starts itself (ADR-0034): `claude -p` with stream-json input
//! and output, in a directory under one of the node's spawn roots, through the user's login
//! shell so it gets a terminal's PATH. The node owns the process's stdin and stdout: a
//! prompt is a `user` line, an interrupt the SDK's `control_request`, kill closes stdin and
//! kills the process if it lingers. Busy and idle come from the stream, since the registry
//! keeps such a session `busy` (EXP-20261106-37f79k); everything else about the session
//! (its transcript, hooks, registry entry) is tracked as for any other.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use rc2s2_proto::event::v1::SessionStatus;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{Mutex, mpsc};

use crate::sessions::Sessions;

/// How long a killed session has to exit after its input closes before it is killed.
const KILL_GRACE: Duration = Duration::from_secs(5);

pub struct Spawner {
    /// The claude binary, resolved by the login shell when it is a bare name.
    claude: String,
    /// The shell that starts it: the user's `$SHELL`, run as a login shell.
    shell: PathBuf,
    /// Canonical directories a session may be spawned under.
    roots: Vec<PathBuf>,
    sessions: Arc<Sessions>,
    /// Running spawned sessions by id: the line writer for each one's stdin. Dropping the
    /// sender closes the input, which ends the session.
    procs: Mutex<HashMap<String, mpsc::Sender<String>>>,
}

impl Spawner {
    /// Roots that do not exist are an error, so a typo in the service's flags shows at start.
    pub fn new(
        claude: String,
        shell: PathBuf,
        roots: &[PathBuf],
        sessions: Arc<Sessions>,
    ) -> std::io::Result<Arc<Self>> {
        let roots = roots
            .iter()
            .map(|r| r.canonicalize())
            .collect::<std::io::Result<_>>()?;
        Ok(Arc::new(Self {
            claude,
            shell,
            roots,
            sessions,
            procs: Mutex::new(HashMap::new()),
        }))
    }

    pub fn roots(&self) -> Vec<String> {
        self.roots
            .iter()
            .map(|r| r.to_string_lossy().into_owned())
            .collect()
    }

    /// The directory a session may be spawned in: a directory under a root, after `~` and
    /// symlinks are resolved. With `create`, the missing part of the path is made, once its
    /// nearest existing parent is known to be under a root and the rest is plain names.
    fn allowed(&self, cwd: &str, create: bool) -> Result<PathBuf, String> {
        if self.roots.is_empty() {
            return Err("spawning is off on this node: start it with --spawn-root".into());
        }
        let expanded = match cwd.strip_prefix("~/") {
            Some(rest) => std::env::var_os("HOME")
                .map(|h| PathBuf::from(h).join(rest))
                .ok_or("HOME is not set")?,
            None => PathBuf::from(cwd),
        };
        if !expanded.is_absolute() {
            return Err(format!("{cwd} is not an absolute path"));
        }
        let base = expanded
            .ancestors()
            .find(|a| a.exists())
            .ok_or_else(|| format!("{cwd}: no part of it exists"))?;
        let missing = expanded.strip_prefix(base).expect("an ancestor").to_path_buf();
        if !missing.as_os_str().is_empty() {
            if !create {
                return Err(format!("{cwd} does not exist"));
            }
            if !missing
                .components()
                .all(|c| matches!(c, std::path::Component::Normal(_)))
            {
                return Err(format!("{cwd}: a directory to create may not contain . or .."));
            }
            let base = base.canonicalize().map_err(|e| format!("{cwd}: {e}"))?;
            if !self.roots.iter().any(|r| base.starts_with(r)) {
                return Err(format!("{cwd} is not under a spawn root of this node"));
            }
            std::fs::create_dir_all(base.join(&missing)).map_err(|e| format!("creating {cwd}: {e}"))?;
            tracing::info!(dir = %base.join(&missing).display(), "created a directory to spawn in");
        }
        let dir = expanded
            .canonicalize()
            .map_err(|e| format!("{cwd}: {e}"))?;
        if !dir.is_dir() {
            return Err(format!("{cwd} is not a directory"));
        }
        if !self.roots.iter().any(|r| dir.starts_with(r)) {
            return Err(format!("{cwd} is not under a spawn root of this node"));
        }
        Ok(dir)
    }

    /// Starts a session with `id` in `cwd`, creating the directory first when `create` asks,
    /// and sends it its first prompt if there is one; without, it waits idle for a prompt.
    pub async fn spawn(
        self: &Arc<Self>,
        id: &str,
        cwd: &str,
        prompt: &str,
        model: &str,
        create: bool,
    ) -> Result<(), String> {
        if uuid::Uuid::parse_str(id).is_err() {
            return Err("the session id is not a UUID".into());
        }
        let dir = self.allowed(cwd, create)?;
        self.start(id, &dir, prompt, model, "--session-id").await
    }

    /// Continues the finished session `id` headless in its directory `cwd`, in place: the
    /// same id and transcript, its context kept (EXP-20261106-62st4b). Refused while the
    /// session runs, and when this node has no transcript of it there.
    pub async fn resume(self: &Arc<Self>, id: &str, cwd: &str, prompt: &str, model: &str) -> Result<(), String> {
        if uuid::Uuid::parse_str(id).is_err() {
            return Err("the session id is not a UUID".into());
        }
        if self.sessions.is_live(id).await || self.procs.lock().await.contains_key(id) {
            return Err("the session is still running".into());
        }
        let dir = self.allowed(cwd, false)?;
        let transcript = crate::sessions::transcript_path(self.sessions.claude_dir(), &dir.to_string_lossy(), id);
        if !transcript.exists() {
            return Err(format!("this node has no transcript of the session in {cwd}"));
        }
        self.start(id, &dir, prompt, model, "--resume").await
    }

    /// Starts claude for session `id` in `dir`, as a new session (`--session-id`) or a
    /// resumed one (`--resume`), and sends the first prompt if there is one.
    async fn start(
        self: &Arc<Self>,
        id: &str,
        dir: &Path,
        prompt: &str,
        model: &str,
        how: &str,
    ) -> Result<(), String> {
        let mut args = vec![
            "-p",
            "--input-format",
            "stream-json",
            "--output-format",
            "stream-json",
            "--verbose",
            how,
            id,
        ];
        if !model.is_empty() {
            args.extend(["--model", model]);
        }
        let mut child = tokio::process::Command::new(&self.shell)
            .arg("-l")
            .arg("-c")
            .arg(r#"exec "$0" "$@""#)
            .arg(&self.claude)
            .args(&args)
            .current_dir(dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("starting {}: {e}", self.shell.display()))?;
        let pid = child.id().unwrap_or_default();
        let mut stdin = child.stdin.take().expect("piped");
        let stdout = child.stdout.take().expect("piped");
        let stderr = child.stderr.take().expect("piped");
        tracing::info!(session = %id, pid, cwd = %dir.display(), "spawned a session");

        let (tx, mut lines) = mpsc::channel::<String>(64);
        let first = !prompt.trim().is_empty();
        if first {
            tx.send(user_line(prompt)).await.expect("just made");
        }
        self.procs.lock().await.insert(id.to_string(), tx);
        self.sessions
            .add_spawned(id, pid, &dir.to_string_lossy(), first)
            .await;

        // The writer: lines to stdin until every sender is gone, then the input closes and a
        // process that does not exit within the grace period is killed.
        let (closed_tx, closed) = tokio::sync::oneshot::channel::<()>();
        tokio::spawn(async move {
            while let Some(line) = lines.recv().await {
                if stdin.write_all(line.as_bytes()).await.is_err() || stdin.flush().await.is_err() {
                    break;
                }
            }
            drop(stdin);
            let _ = closed_tx.send(());
        });
        let sid = id.to_string();
        tokio::spawn(async move {
            let mut err = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = err.next_line().await {
                tracing::info!(session = %sid, "claude: {line}");
            }
        });
        let this = self.clone();
        let sid = id.to_string();
        tokio::spawn(async move {
            let mut out = BufReader::new(stdout).lines();
            let reader = async {
                while let Ok(Some(line)) = out.next_line().await {
                    let Ok(msg) = serde_json::from_str::<Value>(&line) else {
                        continue;
                    };
                    if msg["type"] == "result" {
                        this.sessions.set_status(&sid, SessionStatus::Idle).await;
                    }
                }
            };
            let killer = async {
                if closed.await.is_ok() {
                    tokio::time::sleep(KILL_GRACE).await;
                    true
                } else {
                    false
                }
            };
            tokio::select! {
                _ = reader => {}
                true = killer => {
                    tracing::info!(session = %sid, "the session outlived its closed input; killing it");
                    let _ = child.start_kill();
                }
            }
            let status = child.wait().await;
            tracing::info!(session = %sid, ?status, "spawned session exited");
            this.procs.lock().await.remove(&sid);
            this.sessions.finish(&sid).await;
        });
        Ok(())
    }

    async fn send(&self, id: &str, line: String) -> Result<(), String> {
        let tx = self.procs.lock().await.get(id).cloned();
        let Some(tx) = tx else {
            return Err("the spawned session is not running".into());
        };
        tx.send(line)
            .await
            .map_err(|_| "the spawned session is not running".to_string())
    }

    pub async fn prompt(&self, id: &str, text: &str) -> Result<(), String> {
        if text.trim().is_empty() {
            return Err("the prompt is empty".into());
        }
        self.send(id, user_line(text)).await?;
        self.sessions.set_status(id, SessionStatus::Busy).await;
        Ok(())
    }

    /// Stops the running turn; a follow-up prompt, if any, starts the next one.
    pub async fn interrupt(&self, id: &str, then_prompt: Option<&str>) -> Result<(), String> {
        self.send(id, interrupt_line()).await?;
        if let Some(text) = then_prompt.filter(|t| !t.trim().is_empty()) {
            self.prompt(id, text).await?;
        }
        Ok(())
    }

    /// Interrupts the running turn and closes the session's input, which ends it; the
    /// process is killed if it lingers. Without the interrupt, claude finishes the turn
    /// before it reads the end of its input.
    pub async fn kill(&self, id: &str) -> Result<(), String> {
        let Some(tx) = self.procs.lock().await.remove(id) else {
            return Err("the spawned session is not running".into());
        };
        let _ = tx.send(interrupt_line()).await;
        Ok(())
    }
}

/// The SDK's interrupt: ends the running turn and its tool (EXP-20261106-37f79k).
fn interrupt_line() -> String {
    let request = json!({
        "type": "control_request",
        "request_id": uuid::Uuid::new_v4().simple().to_string(),
        "request": {"subtype": "interrupt"},
    });
    format!("{request}\n")
}

fn user_line(text: &str) -> String {
    let line = json!({"type": "user", "message": {"role": "user", "content": text}});
    format!("{line}\n")
}

/// The login shell to start sessions through: `$SHELL`, else `/bin/sh`.
pub fn login_shell() -> PathBuf {
    std::env::var_os("SHELL")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| Path::new("/bin/sh").to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::outbox::Outbox;
    use rc2s2_proto::event::v1::Launch;
    use rc2s2_proto::node::v1::node_message::Body;

    /// A stand-in for claude: logs its arguments and every input line, answers each `user`
    /// line with a `result`, and exits when its input closes.
    const FAKE: &str = r#"#!/bin/sh
echo "$@" > LOG
while IFS= read -r line; do
  echo "$line" >> LOG
  case "$line" in
    *'"type":"user"'*) echo '{"type":"result","subtype":"success"}' ;;
  esac
done
echo closed >> LOG
"#;

    struct Fixture {
        _dir: tempfile::TempDir,
        root: PathBuf,
        log: PathBuf,
        spawner: Arc<Spawner>,
        sessions: Arc<Sessions>,
        rx: mpsc::Receiver<rc2s2_proto::node::v1::NodeMessage>,
    }

    fn fixture() -> Fixture {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("work");
        std::fs::create_dir_all(root.join("repo")).unwrap();
        let log = dir.path().join("log");
        let fake = dir.path().join("claude");
        let script = FAKE.replace("LOG", &format!("'{}'", log.display()));
        std::fs::write(&fake, script).unwrap();
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
        let (outbox, rx) = Outbox::new(256);
        let sessions = Arc::new(Sessions::new(
            dir.path().join(".claude"),
            outbox,
            Duration::from_millis(20),
            Vec::new(),
        ));
        let spawner = Spawner::new(
            fake.to_string_lossy().into_owned(),
            PathBuf::from("/bin/sh"),
            std::slice::from_ref(&root),
            sessions.clone(),
        )
        .unwrap();
        Fixture {
            root: root.canonicalize().unwrap(),
            _dir: dir,
            log,
            spawner,
            sessions,
            rx,
        }
    }

    async fn log_has(log: &Path, text: &str) -> String {
        for _ in 0..200 {
            let s = std::fs::read_to_string(log).unwrap_or_default();
            if s.contains(text) {
                return s;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("{text:?} never reached {}", log.display());
    }

    async fn status_of(sessions: &Sessions, id: &str) -> (SessionStatus, bool) {
        let s = sessions.snapshot().await;
        let s = s.iter().find(|s| s.id == id).unwrap();
        (s.status(), s.ended_ms.is_some())
    }

    #[tokio::test]
    async fn a_spawned_session_takes_prompts_interrupts_and_a_kill_over_its_input() {
        let mut f = fixture();
        let id = uuid::Uuid::new_v4().to_string();
        let cwd = f.root.join("repo");
        f.spawner
            .spawn(&id, &cwd.to_string_lossy(), "hello", "haiku", false)
            .await
            .unwrap();
        let log = log_has(&f.log, "hello").await;
        let args = log.lines().next().unwrap();
        assert!(args.contains(&format!("--session-id {id}")), "{args}");
        assert!(args.contains("--input-format stream-json --output-format stream-json"));
        assert!(args.ends_with("--model haiku"), "{args}");

        let first = loop {
            match f.rx.recv().await.unwrap().body {
                Some(Body::Session(s)) => break s,
                _ => continue,
            }
        };
        assert_eq!(first.launch(), Launch::Spawned);
        assert_eq!(first.cwd, cwd.to_string_lossy());

        // The fake answers each turn with a result, which marks the session idle.
        for _ in 0..100 {
            if status_of(&f.sessions, &id).await.0 == SessionStatus::Idle {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert_eq!(status_of(&f.sessions, &id).await.0, SessionStatus::Idle);
        assert!(f.sessions.is_spawned(&id).await);

        f.spawner.prompt(&id, "second").await.unwrap();
        f.spawner.interrupt(&id, Some("third")).await.unwrap();
        let log = log_has(&f.log, "third").await;
        let interrupt = log.lines().find(|l| l.contains("control_request")).unwrap();
        let v: Value = serde_json::from_str(interrupt).unwrap();
        assert_eq!(v["request"]["subtype"], "interrupt");
        assert!(log.find("second").unwrap() < log.find("control_request").unwrap());

        f.spawner.kill(&id).await.unwrap();
        let log = log_has(&f.log, "closed").await;
        let last_two: Vec<_> = log.lines().rev().take(2).collect();
        assert!(last_two[1].contains("interrupt"), "a kill interrupts first: {last_two:?}");
        for _ in 0..100 {
            if status_of(&f.sessions, &id).await.1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(status_of(&f.sessions, &id).await.1, "ended once the process exited");
        assert!(f.spawner.prompt(&id, "late").await.is_err());
    }

    #[tokio::test]
    async fn only_directories_under_a_root_are_allowed() {
        let f = fixture();
        let id = uuid::Uuid::new_v4().to_string();
        let outside = f.root.parent().unwrap().to_string_lossy().into_owned();
        let escape = format!("{}/repo/../..", f.root.display());
        for cwd in [outside.as_str(), escape.as_str(), "relative/path", "/no/such/dir"] {
            assert!(f.spawner.spawn(&id, cwd, "hi", "", true).await.is_err(), "{cwd}");
        }
        assert!(
            f.spawner
                .spawn("not-a-uuid", &f.root.to_string_lossy(), "hi", "", false)
                .await
                .is_err()
        );
        let off = Spawner::new("claude".into(), "/bin/sh".into(), &[], f.sessions.clone()).unwrap();
        let err = off
            .spawn(&id, &f.root.to_string_lossy(), "hi", "", false)
            .await
            .unwrap_err();
        assert!(err.contains("--spawn-root"), "{err}");
    }

    #[tokio::test]
    async fn a_missing_directory_is_made_only_when_asked_and_only_under_a_root() {
        let f = fixture();
        let new = f.root.join("new/deeper");
        let path = new.to_string_lossy().into_owned();
        let id = uuid::Uuid::new_v4().to_string();
        let err = f.spawner.spawn(&id, &path, "", "", false).await.unwrap_err();
        assert!(err.contains("does not exist"), "{err}");
        assert!(!new.exists());

        let outside = f.root.parent().unwrap().join("elsewhere/new");
        let err = f
            .spawner
            .spawn(&id, &outside.to_string_lossy(), "", "", true)
            .await
            .unwrap_err();
        assert!(err.contains("spawn root"), "{err}");
        assert!(!outside.parent().unwrap().exists());
        let sneaky = format!("{}/new/../../elsewhere", f.root.display());
        assert!(f.spawner.spawn(&id, &sneaky, "", "", true).await.is_err());

        // Made, then started idle with no prompt: nothing written to its input.
        f.spawner.spawn(&id, &path, "", "", true).await.unwrap();
        assert!(new.is_dir());
        let log = log_has(&f.log, "--session-id").await;
        assert_eq!(log.lines().count(), 1, "no first prompt: {log}");
        assert_eq!(status_of(&f.sessions, &id).await.0, SessionStatus::Idle);
        f.spawner.kill(&id).await.unwrap();
    }

    #[tokio::test]
    async fn a_finished_session_resumes_in_place_and_only_when_finished() {
        let f = fixture();
        let id = uuid::Uuid::new_v4().to_string();
        let cwd = f.root.join("repo");
        let cwd_s = cwd.to_string_lossy().into_owned();
        let err = f.spawner.resume(&id, &cwd_s, "", "").await.unwrap_err();
        assert!(err.contains("no transcript"), "{err}");

        let transcript = crate::sessions::transcript_path(f.sessions.claude_dir(), &cwd_s, &id);
        std::fs::create_dir_all(transcript.parent().unwrap()).unwrap();
        std::fs::write(&transcript, "").unwrap();
        f.spawner.resume(&id, &cwd_s, "go on", "").await.unwrap();
        let log = log_has(&f.log, "go on").await;
        let args = log.lines().next().unwrap();
        assert!(args.contains(&format!("--resume {id}")), "{args}");
        assert!(!args.contains("--session-id"), "{args}");
        assert!(f.sessions.is_spawned(&id).await);

        let err = f.spawner.resume(&id, &cwd_s, "", "").await.unwrap_err();
        assert!(err.contains("still running"), "{err}");
        f.spawner.kill(&id).await.unwrap();
    }
}
