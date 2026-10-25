//! The rc2s2 client core: everything a client does against the hub's control service, in
//! Rust, exposed to Kotlin (and later Swift) through UniFFI (ADR-0009). The app keeps the
//! hub's address and its token itself; the core holds the connection, keeps lists and
//! streams typed, and turns events into a conversation.

pub mod conversation;

use std::collections::HashMap;
use std::sync::Arc;

use rc2s2_proto::control::v1::control_service_client::ControlServiceClient;
use rc2s2_proto::control::v1::*;
use rc2s2_proto::event::v1 as ev;
use rc2s2_proto::event::v1::resolution::Decision;
use serde_json::Value;
use tokio::sync::watch;
use tonic::transport::Channel;

pub use conversation::{Conversation, Item, ModelTotals, Role};

uniffi::setup_scaffolding!();

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum CoreError {
    /// The hub could not be reached or the connection broke.
    #[error("{reason}")]
    Connection { reason: String },
    /// The hub turned the request away: a wrong token, an unknown request, a node offline.
    #[error("{reason}")]
    Hub { reason: String },
    /// The node refused the command, with its reason, shown to the person as is.
    #[error("{reason}")]
    Refused { reason: String },
}

impl From<tonic::Status> for CoreError {
    fn from(s: tonic::Status) -> Self {
        match s.code() {
            tonic::Code::Unavailable | tonic::Code::Unknown => CoreError::Connection {
                reason: s.message().to_string(),
            },
            _ => CoreError::Hub {
                reason: s.message().to_string(),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct NodeInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub connected: bool,
    pub last_seen_ms: i64,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct SessionInfo {
    pub node_id: String,
    pub id: String,
    pub cwd: String,
    pub pid: u32,
    /// Busy or idle, as Claude Code's registry says.
    pub busy: bool,
    /// Under `rc2s2 run`, so it can be interrupted and killed.
    pub wrapped: bool,
    pub started_ms: i64,
    pub ended_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Question {
    pub question: String,
    pub header: String,
    pub multi_select: bool,
    pub options: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct PendingInfo {
    pub node_id: String,
    pub id: String,
    pub session_id: String,
    pub tool_name: String,
    /// The one input a person decides on: the command, the path, the question.
    pub summary: String,
    pub tool_input_json: String,
    /// The questions of an AskUserQuestion; empty for a permission request.
    pub questions: Vec<Question>,
    pub created_ms: i64,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct MetricValue {
    pub name: String,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct HealthPoint {
    pub time_ms: i64,
    pub metrics: Vec<MetricValue>,
}

#[derive(Debug, Clone, PartialEq, uniffi::Enum)]
pub enum Change {
    Node {
        node: NodeInfo,
    },
    Session {
        session: SessionInfo,
    },
    PendingOpened {
        pending: PendingInfo,
    },
    PendingClosed {
        node_id: String,
        pending_id: String,
        reason: String,
    },
}

/// Receives the hub's changes to nodes, sessions and pending requests.
#[uniffi::export(with_foreign)]
pub trait WatchListener: Send + Sync {
    fn on_change(&self, change: Change);
    /// The stream ended; list again and watch again.
    fn on_error(&self, message: String);
}

/// Receives a session's conversation, stored items first, then live ones.
#[uniffi::export(with_foreign)]
pub trait SessionListener: Send + Sync {
    /// Items in order; `last_seq` is where to resume after an error.
    fn on_items(&self, items: Vec<Item>, last_seq: u64);
    /// The session's token totals by model, whenever they change.
    fn on_usage(&self, totals: Vec<ModelTotals>);
    fn on_error(&self, message: String);
}

/// A running watch or session stream; dropping it or calling `cancel` stops it.
#[derive(uniffi::Object)]
pub struct Subscription {
    stop: watch::Sender<bool>,
}

#[uniffi::export]
impl Subscription {
    pub fn cancel(&self) {
        let _ = self.stop.send(true);
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        let _ = self.stop.send(true);
    }
}

#[derive(uniffi::Object)]
pub struct Hub {
    client: ControlServiceClient<Channel>,
    bearer: tonic::metadata::MetadataValue<tonic::metadata::Ascii>,
}

#[uniffi::export(async_runtime = "tokio")]
impl Hub {
    /// Connects to the hub's control service, `http://host:port`, with a client token.
    #[uniffi::constructor]
    pub async fn connect(address: String, token: String) -> Result<Arc<Self>, CoreError> {
        // Trim whitespace a paste or an on-screen keyboard may add: a trailing
        // newline or stray space in the token otherwise fails to parse as a
        // header value, and in the address breaks the endpoint URL.
        let address = address.trim().to_string();
        let bearer = format!("Bearer {}", token.trim())
            .parse()
            .map_err(|_| CoreError::Hub {
                reason: "the token is not valid in a header".into(),
            })?;
        // A phone's link dies silently (sleep, a network change, the tailnet relay
        // moving): HTTP/2 pings notice within half a minute and the streams end with an
        // error the app reconnects on, and a connect attempt gives up in seconds rather
        // than hanging until the OS does.
        let channel = tonic::transport::Endpoint::from_shared(address.clone())
            .map_err(|e| CoreError::Connection {
                reason: format!("{address}: {e}"),
            })?
            .connect_timeout(std::time::Duration::from_secs(6))
            .tcp_keepalive(Some(std::time::Duration::from_secs(30)))
            .http2_keep_alive_interval(std::time::Duration::from_secs(15))
            .keep_alive_timeout(std::time::Duration::from_secs(10))
            .keep_alive_while_idle(true)
            .connect()
            .await
            .map_err(|e| CoreError::Connection {
                reason: format!("{address}: {e}"),
            })?;
        let hub = Self {
            client: ControlServiceClient::new(channel),
            bearer,
        };
        // Fail now on a wrong token rather than at the first screen.
        hub.nodes().await?;
        Ok(Arc::new(hub))
    }

    pub async fn nodes(&self) -> Result<Vec<NodeInfo>, CoreError> {
        let r = self
            .client
            .clone()
            .list_nodes(self.req(ListNodesRequest {}))
            .await?;
        Ok(r.into_inner().nodes.into_iter().map(node).collect())
    }

    /// Live sessions, or every session with `include_ended`.
    pub async fn sessions(&self, include_ended: bool) -> Result<Vec<SessionInfo>, CoreError> {
        let req = ListSessionsRequest {
            node_id: String::new(),
            include_ended,
        };
        let r = self.client.clone().list_sessions(self.req(req)).await?;
        Ok(r.into_inner().sessions.into_iter().map(session).collect())
    }

    pub async fn pending(&self) -> Result<Vec<PendingInfo>, CoreError> {
        let r = self
            .client
            .clone()
            .list_pending(self.req(ListPendingRequest {}))
            .await?;
        Ok(r.into_inner()
            .pending
            .into_iter()
            .filter_map(|p| Some(pending(p.node_id, p.request?)))
            .collect())
    }

    /// Health samples of a node since a time.
    pub async fn health(
        &self,
        node_id: String,
        since_ms: i64,
    ) -> Result<Vec<HealthPoint>, CoreError> {
        let req = GetHealthRequest { node_id, since_ms };
        let r = self.client.clone().get_health(self.req(req)).await?;
        Ok(r.into_inner()
            .samples
            .into_iter()
            .map(|s| HealthPoint {
                time_ms: s.time_ms,
                metrics: s
                    .metrics
                    .into_iter()
                    .map(|m| MetricValue {
                        name: m.name,
                        value: m.value,
                    })
                    .collect(),
            })
            .collect())
    }

    /// Allows a held permission request. Returns the node's note, if any.
    pub async fn allow(&self, node_id: String, pending_id: String) -> Result<String, CoreError> {
        self.resolve(node_id, pending_id, Decision::Allow(ev::Allow {}))
            .await
    }

    /// Denies a held permission request; Claude sees the message.
    pub async fn deny(
        &self,
        node_id: String,
        pending_id: String,
        message: String,
    ) -> Result<String, CoreError> {
        self.resolve(node_id, pending_id, Decision::Deny(ev::Deny { message }))
            .await
    }

    /// Answers a held question: for each question text, the chosen labels or one free-text
    /// answer.
    pub async fn answer(
        &self,
        node_id: String,
        pending_id: String,
        answers: HashMap<String, Vec<String>>,
    ) -> Result<String, CoreError> {
        let answers = ev::Answers {
            answers: answers
                .into_iter()
                .map(|(q, values)| (q, ev::AnswerValue { values }))
                .collect(),
        };
        self.resolve(node_id, pending_id, Decision::Answers(answers))
            .await
    }

    pub async fn prompt(
        &self,
        node_id: String,
        session_id: String,
        text: String,
    ) -> Result<String, CoreError> {
        let req = PromptRequest {
            node_id,
            session_id,
            text,
        };
        outcome(
            self.client
                .clone()
                .prompt(self.req(req))
                .await?
                .into_inner(),
        )
    }

    pub async fn interrupt(
        &self,
        node_id: String,
        session_id: String,
        then_prompt: Option<String>,
    ) -> Result<String, CoreError> {
        let req = InterruptRequest {
            node_id,
            session_id,
            then_prompt,
        };
        outcome(
            self.client
                .clone()
                .interrupt(self.req(req))
                .await?
                .into_inner(),
        )
    }

    pub async fn kill(&self, node_id: String, session_id: String) -> Result<String, CoreError> {
        let req = KillRequest {
            node_id,
            session_id,
        };
        outcome(self.client.clone().kill(self.req(req)).await?.into_inner())
    }

    /// Has the hub post notifications to a UnifiedPush endpoint (ADR-0026).
    pub async fn register_push(&self, endpoint: String) -> Result<(), CoreError> {
        let req = RegisterPushRequest {
            endpoint,
            unregister: false,
        };
        self.client.clone().register_push(self.req(req)).await?;
        Ok(())
    }

    pub async fn unregister_push(&self, endpoint: String) -> Result<(), CoreError> {
        let req = RegisterPushRequest {
            endpoint,
            unregister: true,
        };
        self.client.clone().register_push(self.req(req)).await?;
        Ok(())
    }

    /// Calls the listener with every change to nodes, sessions and pending requests until
    /// the subscription is cancelled. List first, then watch.
    pub async fn watch(
        &self,
        listener: Arc<dyn WatchListener>,
    ) -> Result<Arc<Subscription>, CoreError> {
        let mut stream = self
            .client
            .clone()
            .watch(self.req(WatchRequest {}))
            .await?
            .into_inner();
        let (stop, mut stopped) = watch::channel(false);
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = stopped.changed() => return,
                    msg = stream.message() => match msg {
                        Ok(Some(u)) => {
                            if let Some(c) = u.body.and_then(change) {
                                listener.on_change(c);
                            }
                        }
                        Ok(None) => return listener.on_error("the hub closed the stream".into()),
                        Err(s) => return listener.on_error(s.message().to_string()),
                    },
                }
            }
        });
        Ok(Arc::new(Subscription { stop }))
    }

    /// Calls the listener with a session's conversation after `after_seq`: what is stored,
    /// then what happens, until the subscription is cancelled. `totals` seeds the usage
    /// state when resuming after a cached part, so the next turn's line counts only its own
    /// tokens.
    pub async fn stream_session(
        &self,
        node_id: String,
        session_id: String,
        after_seq: u64,
        totals: Vec<ModelTotals>,
        listener: Arc<dyn SessionListener>,
    ) -> Result<Arc<Subscription>, CoreError> {
        let req = StreamSessionRequest {
            node_id,
            session_id,
            after_seq,
        };
        let mut stream = self
            .client
            .clone()
            .stream_session(self.req(req))
            .await?
            .into_inner();
        let (stop, mut stopped) = watch::channel(false);
        tokio::spawn(async move {
            let mut conversation = Conversation::seeded(totals);
            loop {
                tokio::select! {
                    _ = stopped.changed() => return,
                    msg = stream.message() => match msg {
                        Ok(Some(e)) => {
                            let items = conversation.push(&e);
                            if !items.is_empty() {
                                listener.on_items(items, e.seq);
                            }
                            if let Some(totals) = conversation.take_totals() {
                                listener.on_usage(totals);
                            }
                        }
                        Ok(None) => return listener.on_error("the hub closed the stream".into()),
                        Err(s) => return listener.on_error(s.message().to_string()),
                    },
                }
            }
        });
        Ok(Arc::new(Subscription { stop }))
    }
}

impl Hub {
    fn req<T>(&self, body: T) -> tonic::Request<T> {
        let mut r = tonic::Request::new(body);
        r.metadata_mut()
            .insert("authorization", self.bearer.clone());
        r
    }

    async fn resolve(
        &self,
        node_id: String,
        pending_id: String,
        decision: Decision,
    ) -> Result<String, CoreError> {
        let req = ResolveRequest {
            node_id,
            resolution: Some(ev::Resolution {
                pending_id,
                decision: Some(decision),
            }),
        };
        outcome(
            self.client
                .clone()
                .resolve(self.req(req))
                .await?
                .into_inner(),
        )
    }
}

fn outcome(r: CommandResponse) -> Result<String, CoreError> {
    if r.ok {
        Ok(r.note)
    } else {
        Err(CoreError::Refused { reason: r.error })
    }
}

fn node(n: Node) -> NodeInfo {
    NodeInfo {
        id: n.id,
        name: n.name,
        version: n.version,
        connected: n.connected,
        last_seen_ms: n.last_seen_ms,
    }
}

fn session(s: ev::Session) -> SessionInfo {
    SessionInfo {
        busy: s.status() == ev::SessionStatus::Busy,
        wrapped: s.launch() == ev::Launch::Wrapper,
        node_id: s.node_id,
        id: s.id,
        cwd: s.cwd,
        pid: s.pid,
        started_ms: s.started_ms,
        ended_ms: s.ended_ms,
    }
}

fn pending(node_id: String, p: ev::PendingRequest) -> PendingInfo {
    let input: Value = serde_json::from_str(&p.tool_input_json).unwrap_or_default();
    let questions = input["questions"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|q| Question {
            question: q["question"].as_str().unwrap_or_default().into(),
            header: q["header"].as_str().unwrap_or_default().into(),
            multi_select: q["multiSelect"].as_bool().unwrap_or(false),
            options: q["options"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|o| o["label"].as_str().map(str::to_string))
                .collect(),
        })
        .collect();
    PendingInfo {
        summary: conversation::key_input(&input).unwrap_or_default(),
        node_id,
        id: p.id,
        session_id: p.session_id,
        tool_name: p.tool_name,
        tool_input_json: p.tool_input_json,
        questions,
        created_ms: p.created_ms,
    }
}

fn change(body: update::Body) -> Option<Change> {
    Some(match body {
        update::Body::Node(n) => Change::Node { node: node(n) },
        update::Body::Session(s) => Change::Session {
            session: session(s),
        },
        update::Body::Pending(p) => Change::PendingOpened {
            pending: pending(p.node_id, p.request?),
        },
        update::Body::PendingClosed(c) => {
            let closed = c.closed?;
            Change::PendingClosed {
                node_id: c.node_id,
                reason: closed.reason().as_str_name().to_string(),
                pending_id: closed.pending_id,
            }
        }
    })
}
