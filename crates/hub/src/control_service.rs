//! The client-facing API: the hub's view across every node, a session's events as they
//! happen, health history, and the commands a person sends from the phone. Clients
//! authenticate with a pre-shared token, as nodes do, from their own token file.

use std::pin::Pin;

use rc2s2_proto::control::v1::control_service_server::ControlService;
use rc2s2_proto::control::v1::*;
use rc2s2_proto::event::v1::{Event, resolution};
use rc2s2_proto::node::v1::{CommandResult, Interrupt, Kill, Prompt, command};
use tokio::sync::broadcast::error::RecvError;
use tokio_stream::Stream;
use tonic::{Request, Response, Status};

use crate::node_service::Hub;

type Stream_<T> = Pin<Box<dyn Stream<Item = Result<T, Status>> + Send>>;

impl Hub {
    fn client<T>(&self, request: &Request<T>) -> Result<String, Status> {
        let auth = request
            .metadata()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        self.client_tokens
            .name_for(auth)
            .map(str::to_string)
            .ok_or_else(|| Status::unauthenticated("unknown client token"))
    }
}

fn internal(e: anyhow::Error) -> Status {
    tracing::warn!(error = %format!("{e:#}"), "control request failed");
    Status::internal("the hub could not read its record")
}

fn response(r: CommandResult) -> Response<CommandResponse> {
    Response::new(CommandResponse {
        ok: r.ok,
        error: r.error,
        note: r.note,
    })
}

#[tonic::async_trait]
impl ControlService for Hub {
    async fn list_nodes(
        &self,
        request: Request<ListNodesRequest>,
    ) -> Result<Response<ListNodesResponse>, Status> {
        self.client(&request)?;
        let mut nodes = Vec::new();
        for (id, name, version, last_seen_ms) in self.db.nodes().await.map_err(internal)? {
            let connected = self.live.is_linked(&id).await;
            nodes.push(Node {
                id,
                name,
                version,
                connected,
                last_seen_ms,
            });
        }
        Ok(Response::new(ListNodesResponse { nodes }))
    }

    async fn list_sessions(
        &self,
        request: Request<ListSessionsRequest>,
    ) -> Result<Response<ListSessionsResponse>, Status> {
        self.client(&request)?;
        let r = request.into_inner();
        let sessions = self
            .db
            .sessions(&r.node_id, r.include_ended)
            .await
            .map_err(internal)?;
        Ok(Response::new(ListSessionsResponse { sessions }))
    }

    async fn list_pending(
        &self,
        request: Request<ListPendingRequest>,
    ) -> Result<Response<ListPendingResponse>, Status> {
        self.client(&request)?;
        let pending = self
            .db
            .open_pendings()
            .await
            .map_err(internal)?
            .into_iter()
            .map(|(node_id, request)| PendingOnNode {
                node_id,
                request: Some(request),
            })
            .collect();
        Ok(Response::new(ListPendingResponse { pending }))
    }

    type WatchStream = Stream_<Update>;

    async fn watch(
        &self,
        request: Request<WatchRequest>,
    ) -> Result<Response<Self::WatchStream>, Status> {
        let client = self.client(&request)?;
        let mut updates = self.live.subscribe();
        let stream = async_stream(move |tx| async move {
            loop {
                match updates.recv().await {
                    Ok(u) => {
                        if tx.send(Ok(u)).await.is_err() {
                            return;
                        }
                    }
                    // A client too slow for the stream lists again and resubscribes.
                    Err(RecvError::Lagged(n)) => {
                        tracing::info!(%client, missed = n, "watcher lagged");
                        let _ = tx
                            .send(Err(Status::data_loss("missed updates; list again")))
                            .await;
                        return;
                    }
                    Err(RecvError::Closed) => return,
                }
            }
        });
        Ok(Response::new(stream))
    }

    type StreamSessionStream = Stream_<Event>;

    async fn stream_session(
        &self,
        request: Request<StreamSessionRequest>,
    ) -> Result<Response<Self::StreamSessionStream>, Status> {
        self.client(&request)?;
        let r = request.into_inner();
        // Subscribe before reading the record, so nothing stored in between is missed; the
        // live part skips what the stored part already sent.
        let mut live = self.live.subscribe_events();
        let db = self.db.clone();
        let stream = async_stream(move |tx| async move {
            let mut last = r.after_seq;
            loop {
                let stored = match db
                    .session_events(&r.node_id, &r.session_id, last, 500)
                    .await
                {
                    Ok(stored) => stored,
                    Err(e) => {
                        let _ = tx.send(Err(internal(e))).await;
                        return;
                    }
                };
                let done = stored.len() < 500;
                for e in stored {
                    last = e.seq;
                    if tx.send(Ok(e)).await.is_err() {
                        return;
                    }
                }
                if done {
                    break;
                }
            }
            loop {
                match live.recv().await {
                    Ok((node_id, e)) => {
                        if node_id == r.node_id && e.session_id == r.session_id && e.seq > last {
                            last = e.seq;
                            if tx.send(Ok(e)).await.is_err() {
                                return;
                            }
                        }
                    }
                    Err(RecvError::Lagged(_)) => {
                        let _ = tx
                            .send(Err(Status::data_loss(
                                "fell behind; stream again from the last seq",
                            )))
                            .await;
                        return;
                    }
                    Err(RecvError::Closed) => return,
                }
            }
        });
        Ok(Response::new(stream))
    }

    async fn get_health(
        &self,
        request: Request<GetHealthRequest>,
    ) -> Result<Response<GetHealthResponse>, Status> {
        self.client(&request)?;
        let r = request.into_inner();
        let samples = self
            .db
            .health_since(&r.node_id, r.since_ms)
            .await
            .map_err(internal)?;
        Ok(Response::new(GetHealthResponse { samples }))
    }

    async fn resolve(
        &self,
        request: Request<ResolveRequest>,
    ) -> Result<Response<CommandResponse>, Status> {
        let client = self.client(&request)?;
        let r = request.into_inner();
        let resolution = r
            .resolution
            .ok_or_else(|| Status::invalid_argument("no resolution"))?;
        let Some(pending) = self
            .db
            .open_pending(&r.node_id, &resolution.pending_id)
            .await
            .map_err(internal)?
        else {
            return Err(Status::not_found("no such open request"));
        };
        let decision = match &resolution.decision {
            Some(resolution::Decision::Allow(_)) => "allow",
            Some(resolution::Decision::Deny(_)) => "deny",
            Some(resolution::Decision::Answers(_)) => "answers",
            None => "none",
        };
        let result = self
            .live
            .command(
                &r.node_id,
                &pending.session_id,
                command::Action::Resolve(resolution.clone()),
            )
            .await?;
        if result.ok {
            let json = serde_json::json!({ "by": client, "decision": decision }).to_string();
            self.db
                .record_resolution(&r.node_id, &resolution.pending_id, &json)
                .await
                .map_err(internal)?;
        }
        Ok(response(result))
    }

    async fn prompt(
        &self,
        request: Request<PromptRequest>,
    ) -> Result<Response<CommandResponse>, Status> {
        self.client(&request)?;
        let r = request.into_inner();
        let action = command::Action::Prompt(Prompt { text: r.text });
        Ok(response(
            self.live.command(&r.node_id, &r.session_id, action).await?,
        ))
    }

    async fn interrupt(
        &self,
        request: Request<InterruptRequest>,
    ) -> Result<Response<CommandResponse>, Status> {
        self.client(&request)?;
        let r = request.into_inner();
        let action = command::Action::Interrupt(Interrupt {
            then_prompt: r.then_prompt,
        });
        Ok(response(
            self.live.command(&r.node_id, &r.session_id, action).await?,
        ))
    }

    async fn kill(
        &self,
        request: Request<KillRequest>,
    ) -> Result<Response<CommandResponse>, Status> {
        self.client(&request)?;
        let r = request.into_inner();
        let action = command::Action::Kill(Kill {});
        Ok(response(
            self.live.command(&r.node_id, &r.session_id, action).await?,
        ))
    }
}

/// A response stream fed by a task, which ends when the client goes away.
fn async_stream<T, F, Fut>(f: F) -> Stream_<T>
where
    T: Send + 'static,
    F: FnOnce(tokio::sync::mpsc::Sender<Result<T, Status>>) -> Fut,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    let (tx, rx) = tokio::sync::mpsc::channel(64);
    tokio::spawn(f(tx));
    Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx))
}
