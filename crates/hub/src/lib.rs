//! The rc2s2 hub: records every node's sessions, events and health. The control service for
//! clients comes with the client core.

pub mod db;
pub mod node_service;
pub mod tokens;

use anyhow::Result;
use rc2s2_proto::node::v1::node_service_server::NodeServiceServer;
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;

use crate::node_service::Hub;

/// Serves the hub on a listener until the task is dropped. The listener is the transport
/// boundary: the embedded tsnet listener takes its place in phase 3.
pub async fn serve(listener: TcpListener, hub: Hub) -> Result<()> {
    tonic::transport::Server::builder()
        .add_service(NodeServiceServer::new(hub))
        .serve_with_incoming(TcpListenerStream::new(listener))
        .await?;
    Ok(())
}
