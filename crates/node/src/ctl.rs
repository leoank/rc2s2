//! `rc2s2 ctl`: a command-line client of the hub's control service, for trying intervention
//! before the phone app exists and for scripts. A node is named by its name or its hub id.

use std::collections::HashMap;

use anyhow::{Context, Result, bail};
use clap::Subcommand;
use rc2s2_proto::control::v1::control_service_client::ControlServiceClient;
use rc2s2_proto::control::v1::*;
use rc2s2_proto::event::v1::{AnswerValue, Answers, Deny, Resolution, resolution};
use tonic::Request;
use tonic::transport::Channel;

#[derive(Subcommand)]
pub enum Ctl {
    /// Every node the hub knows, connected or not.
    Nodes,
    /// Live sessions, or every session with `--all`.
    Sessions {
        #[arg(long)]
        all: bool,
    },
    /// Open permission requests and questions.
    Pending,
    /// Print changes to nodes, sessions and pending requests until interrupted.
    Watch,
    /// Allow a held permission request.
    Allow { node: String, pending_id: String },
    /// Deny a held permission request; Claude sees the message.
    Deny {
        node: String,
        pending_id: String,
        #[arg(long, default_value = "")]
        message: String,
    },
    /// Answer a held question: `--answer "<question>=<answer>"` once per question, repeated
    /// for each label of a multi-select question.
    Answer {
        node: String,
        pending_id: String,
        #[arg(long = "answer", required = true)]
        answers: Vec<String>,
    },
    /// Send a prompt to a session.
    Prompt {
        node: String,
        session_id: String,
        text: String,
    },
    /// Interrupt a session's turn, optionally typing a prompt after it.
    Interrupt {
        node: String,
        session_id: String,
        #[arg(long)]
        then: Option<String>,
    },
}

struct Client {
    inner: ControlServiceClient<Channel>,
    bearer: tonic::metadata::MetadataValue<tonic::metadata::Ascii>,
}

impl Client {
    fn request<T>(&self, body: T) -> Request<T> {
        let mut r = Request::new(body);
        r.metadata_mut()
            .insert("authorization", self.bearer.clone());
        r
    }

    /// A node's hub id from its name or id.
    async fn node_id(&mut self, node: &str) -> Result<String> {
        let nodes = self
            .inner
            .list_nodes(self.request(ListNodesRequest {}))
            .await?
            .into_inner()
            .nodes;
        match nodes.iter().find(|n| n.id == node || n.name == node) {
            Some(n) => Ok(n.id.clone()),
            None => bail!("no node named {node:?}"),
        }
    }
}

pub async fn run(hub: &str, token: &str, cmd: Ctl) -> Result<()> {
    let channel = tonic::transport::Endpoint::from_shared(hub.to_string())
        .context("hub address")?
        .connect()
        .await
        .with_context(|| format!("connecting to {hub}"))?;
    let mut c = Client {
        inner: ControlServiceClient::new(channel),
        bearer: format!("Bearer {token}")
            .parse()
            .context("token is not valid in a header")?,
    };
    match cmd {
        Ctl::Nodes => {
            let r = c.inner.list_nodes(c.request(ListNodesRequest {})).await?;
            for n in r.into_inner().nodes {
                let state = if n.connected { "connected" } else { "offline" };
                println!("{}  {}  {}  {state}", n.id, n.name, n.version);
            }
        }
        Ctl::Sessions { all } => {
            let req = ListSessionsRequest {
                node_id: String::new(),
                include_ended: all,
            };
            let r = c.inner.list_sessions(c.request(req)).await?;
            for s in r.into_inner().sessions {
                let ended = if s.ended_ms.is_some() { "  ended" } else { "" };
                println!(
                    "{}  {}  pid {}  {}  {}{ended}",
                    s.node_id,
                    s.id,
                    s.pid,
                    s.status().as_str_name(),
                    s.cwd
                );
            }
        }
        Ctl::Pending => {
            let r = c
                .inner
                .list_pending(c.request(ListPendingRequest {}))
                .await?;
            for p in r.into_inner().pending {
                let q = p.request.unwrap_or_default();
                println!(
                    "{}  {}  {}  {}  {}  {}",
                    p.node_id,
                    q.id,
                    q.session_id,
                    q.kind().as_str_name(),
                    q.tool_name,
                    q.tool_input_json
                );
            }
        }
        Ctl::Watch => {
            let mut s = c
                .inner
                .watch(c.request(WatchRequest {}))
                .await?
                .into_inner();
            while let Some(u) = s.message().await? {
                println!("{:?}", u.body);
            }
        }
        Ctl::Allow { node, pending_id } => {
            let d = resolution::Decision::Allow(Default::default());
            resolve(&mut c, &node, pending_id, d).await?;
        }
        Ctl::Deny {
            node,
            pending_id,
            message,
        } => {
            let d = resolution::Decision::Deny(Deny { message });
            resolve(&mut c, &node, pending_id, d).await?;
        }
        Ctl::Answer {
            node,
            pending_id,
            answers,
        } => {
            let mut map: HashMap<String, AnswerValue> = HashMap::new();
            for a in answers {
                let Some((q, v)) = a.split_once('=') else {
                    bail!("an answer is \"<question>=<answer>\", got {a:?}");
                };
                map.entry(q.to_string())
                    .or_default()
                    .values
                    .push(v.to_string());
            }
            let d = resolution::Decision::Answers(Answers { answers: map });
            resolve(&mut c, &node, pending_id, d).await?;
        }
        Ctl::Prompt {
            node,
            session_id,
            text,
        } => {
            let node_id = c.node_id(&node).await?;
            let req = PromptRequest {
                node_id,
                session_id,
                text,
            };
            let r = c.inner.prompt(c.request(req)).await?.into_inner();
            report(r)?;
        }
        Ctl::Interrupt {
            node,
            session_id,
            then,
        } => {
            let node_id = c.node_id(&node).await?;
            let req = InterruptRequest {
                node_id,
                session_id,
                then_prompt: then,
            };
            let r = c.inner.interrupt(c.request(req)).await?.into_inner();
            report(r)?;
        }
    }
    Ok(())
}

async fn resolve(
    c: &mut Client,
    node: &str,
    pending_id: String,
    decision: resolution::Decision,
) -> Result<()> {
    let node_id = c.node_id(node).await?;
    let req = ResolveRequest {
        node_id,
        resolution: Some(Resolution {
            pending_id,
            decision: Some(decision),
        }),
    };
    report(c.inner.resolve(c.request(req)).await?.into_inner())
}

fn report(r: CommandResponse) -> Result<()> {
    if !r.ok {
        bail!("the node refused: {}", r.error);
    }
    if r.note.is_empty() {
        println!("ok");
    } else {
        println!("ok: {}", r.note);
    }
    Ok(())
}
