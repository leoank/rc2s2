//! The rc2s2 hub: records every node's sessions, events and health, and passes a person's
//! decisions and prompts from a client to the node that holds the session.

pub mod control_service;
pub mod db;
pub mod live;
pub mod node_service;
pub mod notify;
pub mod tokens;

use anyhow::Result;
use rc2s2_proto::control::v1::control_service_server::ControlServiceServer;
use rc2s2_proto::node::v1::node_service_server::NodeServiceServer;
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;

use crate::node_service::Hub;

/// Serves both services on a listener until the task is dropped. The listener is the
/// transport boundary: the embedded tsnet listener takes its place in phase 3.
pub async fn serve(listener: TcpListener, hub: Hub) -> Result<()> {
    tonic::transport::Server::builder()
        .add_service(NodeServiceServer::new(hub.clone()))
        .add_service(ControlServiceServer::new(hub))
        .serve_with_incoming(TcpListenerStream::new(listener))
        .await?;
    Ok(())
}
