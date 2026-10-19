//! `rc2s2-hub`: records what every node observes.

use std::io::IsTerminal;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;
use rc2s2_hub::db::Db;
use rc2s2_hub::node_service::Hub;
use rc2s2_hub::tokens::Tokens;

#[derive(Parser)]
#[command(
    name = "rc2s2-hub",
    version,
    about = "rc2s2 hub: records every node's sessions, events and health"
)]
struct Cli {
    /// Where nodes connect.
    #[arg(long, default_value = "127.0.0.1:7700")]
    listen: SocketAddr,
    /// The SQLite database; created on first start.
    #[arg(long)]
    db: PathBuf,
    /// A file of `<node name> <token>` lines.
    #[arg(long)]
    node_tokens: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(std::io::stderr().is_terminal())
        .init();
    let cli = Cli::parse();
    let hub = Hub {
        db: Db::open(&cli.db).await?,
        tokens: Arc::new(Tokens::load(&cli.node_tokens)?),
    };
    let listener = tokio::net::TcpListener::bind(cli.listen)
        .await
        .with_context(|| format!("binding {}", cli.listen))?;
    tracing::info!(listen = %cli.listen, db = %cli.db.display(), "hub serving");
    tokio::select! {
        r = rc2s2_hub::serve(listener, hub) => r,
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("stopping");
            Ok(())
        }
    }
}
