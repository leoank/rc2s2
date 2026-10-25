//! A real hub and a real node on loopback, shared by the hub's end-to-end tests.
#![allow(dead_code)]

use std::sync::Arc;
use std::time::Duration;

use rc2s2_hub::db::Db;
use rc2s2_hub::live::Live;
use rc2s2_hub::node_service::Hub;
use rc2s2_hub::tokens::Tokens;
use rc2s2_node::link;
use rc2s2_node::outbox::Outbox;
use rc2s2_node::pending::Pending;
use rc2s2_node::sessions::Sessions;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

pub const TOKENS: &str = "test-node t-node\n";
pub const CLIENT_TOKENS: &str = "test-client t-client\n";

/// A hub on its own runtime in its own thread, so stopping it drops every connection and
/// task it had, as a process exit would.
pub struct HubProc {
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl HubProc {
    pub fn start(db_path: &std::path::Path, addr: &str) -> (Self, String) {
        Self::start_with(db_path, addr, None)
    }

    /// The same, publishing notifications to an ntfy topic URL.
    pub fn start_with(
        db_path: &std::path::Path,
        addr: &str,
        ntfy: Option<String>,
    ) -> (Self, String) {
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let (bound_tx, bound_rx) = std::sync::mpsc::channel();
        let (db_path, addr) = (db_path.to_path_buf(), addr.to_string());
        let thread = std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                let db = Db::open(&db_path).await.unwrap();
                let listener = TcpListener::bind(&addr).await.unwrap();
                bound_tx
                    .send(listener.local_addr().unwrap().to_string())
                    .unwrap();
                let hub = Hub {
                    notifier: Arc::new(rc2s2_hub::notify::Notifier::new(ntfy, db.clone(), Vec::new())),
                    db,
                    node_tokens: Arc::new(Tokens::parse(TOKENS).unwrap()),
                    client_tokens: Arc::new(Tokens::parse(CLIENT_TOKENS).unwrap()),
                    live: Live::new(),
                };
                tokio::select! {
                    r = rc2s2_hub::serve(listener, hub) => r.unwrap(),
                    _ = stopped => {}
                }
            });
            rt.shutdown_timeout(Duration::ZERO);
        });
        let addr = bound_rx.recv().unwrap();
        (
            Self {
                stop: Some(stop),
                thread: Some(thread),
            },
            addr,
        )
    }

    pub fn stop(mut self) {
        let _ = self.stop.take().unwrap().send(());
        self.thread.take().unwrap().join().unwrap();
    }
}

/// A node as `rc2s2 node serve --hub` runs it: its outbox draining into a link to the hub,
/// and its hook endpoint on a free port holding PermissionRequest for `hold`.
pub struct NodeProc {
    pub outbox: Outbox,
    pub sessions: Arc<Sessions>,
    pub pending: Arc<Pending>,
    pub hooks: std::net::SocketAddr,
    pub task: JoinHandle<()>,
}

pub async fn start_node(
    hub_addr: &str,
    token: &str,
    claude_dir: &std::path::Path,
    hold: Duration,
) -> NodeProc {
    let (outbox, rx) = Outbox::new(1024);
    let sessions = Arc::new(Sessions::new(
        claude_dir.to_path_buf(),
        outbox.clone(),
        Duration::from_millis(20),
            Vec::new(),
        ));
    let pending = Pending::new(outbox.clone());
    let cfg = link::Config {
        hub: format!("http://{hub_addr}"),
        token: token.to_string(),
        name: "test-node".to_string(),
        buffer_cap: 10_000,
    };
    let wrappers =
        rc2s2_node::wrapper::Wrappers::new(sessions.clone(), pending.clone(), outbox.clone());
    let task = tokio::spawn(link::run(
        cfg,
        rx,
        sessions.clone(),
        pending.clone(),
        wrappers,
    ));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let hooks = listener.local_addr().unwrap();
    let hold = rc2s2_node::hooks::Hold {
        pending: pending.clone(),
        duration: hold,
        wrapped_duration: hold,
    };
    let app = rc2s2_node::hooks::router(outbox.clone(), sessions.clone(), Some(hold));
    tokio::spawn(async move { axum_serve(listener, app).await });
    NodeProc {
        outbox,
        sessions,
        pending,
        hooks,
        task,
    }
}

async fn axum_serve(listener: TcpListener, app: axum::Router) {
    let _ = axum::serve(listener, app).await;
}

/// Posts a hook input the way Claude Code does and returns the response body.
pub async fn post_hook(addr: std::net::SocketAddr, body: String) -> String {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let request = format!(
        "POST /hook HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut reply = String::new();
    stream.read_to_string(&mut reply).await.unwrap();
    reply
        .split("\r\n\r\n")
        .nth(1)
        .unwrap_or_default()
        .to_string()
}
