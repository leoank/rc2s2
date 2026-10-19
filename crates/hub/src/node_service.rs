//! The daemon-facing `Link` stream: authenticate by token, register, then store what the node
//! sends and ack every event once it is stored. Record only: no commands go out yet.

use std::pin::Pin;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Result, bail};
use rc2s2_proto::node::v1::node_service_server::NodeService;
use rc2s2_proto::node::v1::{
    Ack, HubMessage, NodeMessage, Registered, hub_message, node_message::Body,
};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::{Stream, StreamExt};
use tonic::{Request, Response, Status, Streaming};

use crate::db::Db;
use crate::tokens::Tokens;

#[derive(Clone)]
pub struct Hub {
    pub db: Db,
    pub tokens: Arc<Tokens>,
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
        let Some(name) = self.tokens.node_for(auth).map(str::to_string) else {
            return Err(Status::unauthenticated("unknown node token"));
        };
        let (tx, rx) = mpsc::channel(256);
        let db = self.db.clone();
        let inbound = request.into_inner();
        tokio::spawn(async move {
            if let Err(e) = serve_node(&db, &name, inbound, &tx).await {
                tracing::warn!(node = %name, error = %format!("{e:#}"), "node link ended");
                let _ = tx
                    .send(Err(Status::internal("the hub could not record the stream")))
                    .await;
            }
        });
        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }
}

async fn serve_node(
    db: &Db,
    name: &str,
    mut inbound: Streaming<NodeMessage>,
    tx: &mpsc::Sender<Result<HubMessage, Status>>,
) -> Result<()> {
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
    send(
        tx,
        hub_message::Body::Registered(Registered {
            node_id: node_id.clone(),
            last_seq,
        }),
    )
    .await?;
    tracing::info!(node = %name, %node_id, last_seq, sessions = register.sessions.len(), "node registered");

    while let Some(msg) = inbound.next().await {
        match msg?.body {
            Some(Body::Event(e)) => {
                db.store_event(&node_id, &e).await?;
                send(tx, hub_message::Body::Ack(Ack { seq: e.seq })).await?;
            }
            Some(Body::Session(s)) => db.store_session(&node_id, &s).await?,
            Some(Body::Health(h)) => db.store_health(&node_id, &h).await?,
            Some(Body::Register(_)) => bail!("registered twice on one stream"),
            Some(other) => tracing::debug!(node = %name, ?other, "not recorded until intervention"),
            None => {}
        }
    }
    db.touch_node(&node_id, now_ms()).await?;
    tracing::info!(node = %name, "node disconnected");
    Ok(())
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

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
