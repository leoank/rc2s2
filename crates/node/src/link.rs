//! The hub link: one outbound `Link` stream, reconnected with backoff. Events stay in a
//! buffer until the hub acks them, so a hub restart or an outage loses nothing; on every
//! (re)connect the node registers with its current sessions and resends what the hub has not
//! stored. Sessions and health are not buffered: the registration carries the sessions, and
//! health samples taken while disconnected are dropped.

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use rc2s2_proto::event::v1::Event;
use rc2s2_proto::node::v1::node_service_client::NodeServiceClient;
use rc2s2_proto::node::v1::{NodeMessage, Register, hub_message, node_message::Body};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::Request;
use tonic::transport::{Channel, Endpoint};

use crate::sessions::Sessions;

const MIN_BACKOFF: Duration = Duration::from_secs(1);
const MAX_BACKOFF: Duration = Duration::from_secs(30);

pub struct Config {
    /// The hub's node service, `http://host:port`.
    pub hub: String,
    pub token: String,
    pub name: String,
    /// Unacked events kept at most; the oldest are dropped beyond it.
    pub buffer_cap: usize,
}

/// Runs until the outbox closes, which happens only when the node shuts down.
pub async fn run(cfg: Config, mut rx: mpsc::Receiver<NodeMessage>, sessions: Arc<Sessions>) {
    let mut buffer = Buffer {
        events: VecDeque::new(),
        cap: cfg.buffer_cap,
        dropped: 0,
    };
    let mut backoff = MIN_BACKOFF;
    loop {
        match stream(&cfg, &mut rx, &sessions, &mut buffer, &mut backoff).await {
            Ok(()) => return,
            Err(e) => {
                tracing::warn!(error = %format!("{e:#}"), buffered = buffer.events.len(), "hub link down")
            }
        }
        // Keep draining the outbox while waiting, so hooks never block on a full channel.
        let wait = tokio::time::sleep(backoff);
        tokio::pin!(wait);
        loop {
            tokio::select! {
                _ = &mut wait => break,
                msg = rx.recv() => match msg {
                    Some(msg) => { buffer.keep(msg); }
                    None => return,
                },
            }
        }
        backoff = (backoff * 2).min(MAX_BACKOFF);
    }
}

struct Buffer {
    events: VecDeque<Event>,
    cap: usize,
    dropped: u64,
}

impl Buffer {
    /// Keeps an event until it is acked; returns it as a message to send now. Other messages
    /// pass through unbuffered.
    fn keep(&mut self, msg: NodeMessage) -> NodeMessage {
        if let Some(Body::Event(e)) = &msg.body {
            if self.events.len() == self.cap {
                self.events.pop_front();
                self.dropped += 1;
                if self.dropped.is_power_of_two() {
                    tracing::warn!(
                        dropped = self.dropped,
                        "event buffer full, dropping the oldest"
                    );
                }
            }
            self.events.push_back(e.clone());
        }
        msg
    }

    fn ack(&mut self, seq: u64) {
        while self.events.front().is_some_and(|e| e.seq <= seq) {
            self.events.pop_front();
        }
    }
}

/// The transport boundary: today a plain TCP connection; the embedded tsnet dialer
/// replaces this function in phase 3.
async fn dial(hub: &str) -> Result<Channel> {
    let endpoint = Endpoint::from_shared(hub.to_string()).context("hub address")?;
    endpoint
        .connect()
        .await
        .with_context(|| format!("connecting to {hub}"))
}

/// One connection: register, resend what the hub lacks, then stream live until it breaks.
/// Ok means the outbox closed.
async fn stream(
    cfg: &Config,
    rx: &mut mpsc::Receiver<NodeMessage>,
    sessions: &Sessions,
    buffer: &mut Buffer,
    backoff: &mut Duration,
) -> Result<()> {
    let mut client = NodeServiceClient::new(dial(&cfg.hub).await?);
    let (tx, outbound) = mpsc::channel(256);
    let register = Register {
        name: cfg.name.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        sessions: sessions.snapshot().await,
        pending: Vec::new(),
    };
    tx.send(NodeMessage {
        body: Some(Body::Register(register)),
    })
    .await?;
    let mut request = Request::new(ReceiverStream::new(outbound));
    let bearer = format!("Bearer {}", cfg.token)
        .parse()
        .context("token is not valid in a header")?;
    request.metadata_mut().insert("authorization", bearer);
    let mut inbound = client
        .link(request)
        .await
        .context("opening the link")?
        .into_inner();

    let Some(hub_message::Body::Registered(registered)) =
        inbound.message().await?.and_then(|m| m.body)
    else {
        bail!("the hub did not answer the registration");
    };
    buffer.ack(registered.last_seq);
    tracing::info!(node_id = %registered.node_id, resending = buffer.events.len(), "registered with the hub");
    for e in &buffer.events {
        tx.send(NodeMessage {
            body: Some(Body::Event(e.clone())),
        })
        .await?;
    }
    *backoff = MIN_BACKOFF;

    loop {
        tokio::select! {
            msg = rx.recv() => {
                let Some(msg) = msg else { return Ok(()) };
                tx.send(buffer.keep(msg)).await.map_err(|_| anyhow!("the link closed"))?;
            }
            reply = inbound.message() => match reply?.and_then(|m| m.body) {
                Some(hub_message::Body::Ack(ack)) => buffer.ack(ack.seq),
                Some(hub_message::Body::Command(cmd)) => {
                    tracing::debug!(id = %cmd.id, "commands come with intervention; ignored");
                }
                Some(hub_message::Body::Registered(_)) => bail!("registered twice on one stream"),
                None => bail!("the hub closed the link"),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(seq: u64) -> NodeMessage {
        NodeMessage {
            body: Some(Body::Event(Event {
                seq,
                ..Default::default()
            })),
        }
    }

    #[test]
    fn buffer_drops_acked_and_oldest() {
        let mut b = Buffer {
            events: VecDeque::new(),
            cap: 3,
            dropped: 0,
        };
        for seq in 1..=4 {
            b.keep(event(seq));
        }
        assert_eq!(
            b.events.iter().map(|e| e.seq).collect::<Vec<_>>(),
            [2, 3, 4]
        );
        assert_eq!(b.dropped, 1);
        b.ack(3);
        assert_eq!(b.events.iter().map(|e| e.seq).collect::<Vec<_>>(), [4]);
    }
}
