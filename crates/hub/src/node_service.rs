//! The daemon-facing `Link` stream: authenticate by token, register, then store what the node
//! sends and ack every event once it is stored. Pending requests and sessions are also
//! published to watching clients; commands from clients go out on the same stream
//! (`Live::command`) and their results come back here.

use std::pin::Pin;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Result, bail};
use rc2s2_proto::control::v1::{Node, PendingClosedOnNode, PendingOnNode, Update, update};
use rc2s2_proto::event::v1::{PendingClosed, ReleaseReason, Session};
use rc2s2_proto::node::v1::node_service_server::NodeService;
use rc2s2_proto::node::v1::{
    Ack, HubMessage, NodeMessage, Registered, hub_message, node_message::Body,
};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::{Stream, StreamExt};
use tonic::{Request, Response, Status, Streaming};

use crate::db::Db;
use crate::live::{Live, ToNode};
use crate::notify::Notifier;
use crate::tokens::Tokens;

/// The hub's state, shared by the node and the control service.
#[derive(Clone)]
pub struct Hub {
    pub db: Db,
    pub node_tokens: Arc<Tokens>,
    pub client_tokens: Arc<Tokens>,
    pub live: Arc<Live>,
    /// Push to registered phones and to the ntfy topic, if one is configured.
    pub notifier: Arc<Notifier>,
}

#[tonic::async_trait]
impl NodeService for Hub {
    type LinkStream = Pin<Box<dyn Stream<Item = Result<HubMessage, Status>> + Send>>;

    async fn link(
        &self,
        request: Request<Streaming<NodeMessage>>,
    ) -> Result<Response<Self::LinkStream>, Status> {
        let auth = request
            .metadata()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        let Some(name) = self.tokens_name(auth) else {
            return Err(Status::unauthenticated("unknown node token"));
        };
        let (tx, rx) = mpsc::channel(256);
        let hub = self.clone();
        let inbound = request.into_inner();
        tokio::spawn(async move {
            if let Err(e) = serve_node(&hub, &name, inbound, &tx).await {
                tracing::warn!(node = %name, error = %format!("{e:#}"), "node link ended");
                let _ = tx
                    .send(Err(Status::internal("the hub could not record the stream")))
                    .await;
            }
        });
        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }
}

impl Hub {
    fn tokens_name(&self, auth: &str) -> Option<String> {
        self.node_tokens.name_for(auth).map(str::to_string)
    }
}

async fn serve_node(
    hub: &Hub,
    name: &str,
    mut inbound: Streaming<NodeMessage>,
    tx: &ToNode,
) -> Result<()> {
    let db = &hub.db;
    let Some(Body::Register(register)) = inbound.message().await?.and_then(|m| m.body) else {
        bail!("the first message was not a registration");
    };
    if register.name != name {
        tracing::info!(node = %name, claimed = %register.name, "node registered under another name than its token's");
    }
    let (node_id, last_seq) = db.register_node(name, &register.version, now_ms()).await?;
    for s in &register.sessions {
        db.store_session(&node_id, s).await?;
    }
    let gone = db
        .sync_pending(&node_id, &register.pending, now_ms())
        .await?;
    send(
        tx,
        hub_message::Body::Registered(Registered {
            node_id: node_id.clone(),
            last_seq,
        }),
    )
    .await?;
    let link = hub.live.link(&node_id, tx.clone()).await;
    tracing::info!(node = %name, %node_id, last_seq, sessions = register.sessions.len(),
        pending = register.pending.len(), "node registered");
    publish_node(hub, &node_id, name, &register.version, true);
    for s in register.sessions {
        publish_session(hub, &node_id, s);
    }
    for request in register.pending {
        publish(
            hub,
            update::Body::Pending(PendingOnNode {
                node_id: node_id.clone(),
                request: Some(request),
            }),
        );
    }
    for pending_id in gone {
        let closed = PendingClosed {
            pending_id,
            reason: ReleaseReason::Unspecified.into(),
        };
        publish(
            hub,
            update::Body::PendingClosed(PendingClosedOnNode {
                node_id: node_id.clone(),
                closed: Some(closed),
            }),
        );
    }

    let outcome = serve_link(hub, &node_id, name, &mut inbound, tx).await;
    db.touch_node(&node_id, now_ms()).await?;
    if hub.live.unlink(&node_id, link).await {
        publish_node(hub, &node_id, name, &register.version, false);
    }
    tracing::info!(node = %name, "node disconnected");
    outcome
}

async fn serve_link(
    hub: &Hub,
    node_id: &str,
    name: &str,
    inbound: &mut Streaming<NodeMessage>,
    tx: &ToNode,
) -> Result<()> {
    let db = &hub.db;
    while let Some(msg) = inbound.next().await {
        match msg?.body {
            Some(Body::Event(e)) => {
                let seq = e.seq;
                if db.store_event(node_id, &e).await? {
                    hub.notifier.event(name, node_id, &e);
                    hub.live.publish_event(node_id, e);
                }
                send(tx, hub_message::Body::Ack(Ack { seq })).await?;
            }
            Some(Body::Session(s)) => {
                if db.store_session(node_id, &s).await? {
                    hub.notifier.ended(name, node_id, &s);
                }
                publish_session(hub, node_id, s);
            }
            Some(Body::Health(h)) => {
                db.store_health(node_id, &h).await?;
                hub.notifier.health(name, node_id, &h).await;
            }
            Some(Body::Pending(p)) => {
                db.store_pending(node_id, &p).await?;
                hub.notifier.pending(name, node_id, &p);
                publish(
                    hub,
                    update::Body::Pending(PendingOnNode {
                        node_id: node_id.to_string(),
                        request: Some(p),
                    }),
                );
            }
            Some(Body::PendingClosed(c)) => {
                db.close_pending(node_id, &c.pending_id, c.reason(), now_ms())
                    .await?;
                publish(
                    hub,
                    update::Body::PendingClosed(PendingClosedOnNode {
                        node_id: node_id.to_string(),
                        closed: Some(c),
                    }),
                );
            }
            Some(Body::CommandResult(r)) => hub.live.finish(r).await,
            Some(Body::Register(_)) => bail!("registered twice on one stream"),
            None => tracing::debug!(node = %name, "empty message"),
        }
    }
    Ok(())
}

fn publish(hub: &Hub, body: update::Body) {
    hub.live.publish(Update { body: Some(body) });
}

fn publish_session(hub: &Hub, node_id: &str, mut session: Session) {
    session.node_id = node_id.to_string();
    publish(hub, update::Body::Session(session));
}

fn publish_node(hub: &Hub, node_id: &str, name: &str, version: &str, connected: bool) {
    publish(
        hub,
        update::Body::Node(Node {
            id: node_id.to_string(),
            name: name.to_string(),
            version: version.to_string(),
            connected,
            last_seen_ms: now_ms(),
        }),
    );
}

async fn send(
    tx: &mpsc::Sender<Result<HubMessage, Status>>,
    body: hub_message::Body,
) -> Result<()> {
    if tx.send(Ok(HubMessage { body: Some(body) })).await.is_err() {
        bail!("the node went away");
    }
    Ok(())
}

pub(crate) fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
