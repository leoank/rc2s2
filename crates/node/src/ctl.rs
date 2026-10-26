//! `rc2s2 ctl`: a command-line client of the hub's control service, for trying intervention
//! before the phone app exists and for scripts. A node is named by its name or its hub id.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

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
    /// End a wrapped session's process.
    Kill { node: String, session_id: String },
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

/// `ctl.toml` in the rc2s2 config directory: the hub and the token file, so neither has to
/// be given on every call. A flag or its environment variable wins over the file.
#[derive(Debug, Default, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub hub: Option<String>,
    pub token_file: Option<PathBuf>,
}

/// `$XDG_CONFIG_HOME/rc2s2/ctl.toml`, else `~/.config/rc2s2/ctl.toml`, on macOS too, beside
/// the token files the guide puts in `~/.config/rc2s2`.
pub fn config_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("rc2s2/ctl.toml"))
}

impl Config {
    /// The file at `path`, or an empty config when there is none.
    pub fn load(path: &Path) -> Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(text) => toml::from_str(&text).with_context(|| format!("reading {}", path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e).with_context(|| format!("reading {}", path.display())),
        }
    }

    /// The hub and token file from the flags (or their environment variables), else from
    /// this file; `~/` in the token file is the home directory.
    pub fn resolve(self, hub: Option<String>, token_file: Option<PathBuf>, path: &Path) -> Result<(String, PathBuf)> {
        let missing = |what: &str, key: &str| {
            anyhow::anyhow!("no {what}: pass --{what}, or set `{key}` in {}", path.display())
        };
        let hub = hub.or(self.hub).ok_or_else(|| missing("hub", "hub"))?;
        let token_file = token_file
            .or(self.token_file)
            .ok_or_else(|| missing("token-file", "token_file"))?;
        let token_file = match (token_file.strip_prefix("~"), std::env::var_os("HOME")) {
            (Ok(rest), Some(home)) => PathBuf::from(home).join(rest),
            _ => token_file,
        };
        Ok((hub, token_file))
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
        Ctl::Kill { node, session_id } => {
            let node_id = c.node_id(&node).await?;
            let req = KillRequest {
                node_id,
                session_id,
            };
            report(c.inner.kill(c.request(req)).await?.into_inner())?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_fills_what_the_flags_leave_out() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ctl.toml");
        std::fs::write(&path, "hub = \"http://hub:7700\"\ntoken_file = \"~/.config/rc2s2/client-token\"\n").unwrap();
        let home = PathBuf::from(std::env::var_os("HOME").unwrap());

        let (hub, token) = Config::load(&path).unwrap().resolve(None, None, &path).unwrap();
        assert_eq!(hub, "http://hub:7700");
        assert_eq!(token, home.join(".config/rc2s2/client-token"));

        let (hub, token) = Config::load(&path)
            .unwrap()
            .resolve(Some("http://other:7700".into()), Some("/tmp/t".into()), &path)
            .unwrap();
        assert_eq!((hub.as_str(), token), ("http://other:7700", PathBuf::from("/tmp/t")));
    }

    #[test]
    fn no_config_and_no_flags_names_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ctl.toml");
        assert_eq!(Config::load(&path).unwrap(), Config::default());
        let err = Config::default().resolve(None, Some("/t".into()), &path).unwrap_err().to_string();
        assert!(err.contains("--hub") && err.contains("ctl.toml"), "{err}");
        std::fs::write(&path, "hubb = \"x\"\n").unwrap();
        assert!(Config::load(&path).is_err());
    }
}

