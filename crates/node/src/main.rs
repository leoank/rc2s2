//! `rc2s2`: the node side of rc2s2. `rc2s2 node serve` runs the daemon; `rc2s2 node hooks`
//! prints the settings block that points Claude Code's hooks at it.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};

use rc2s2_node::outbox::Outbox;
use rc2s2_node::sessions::Sessions;
use rc2s2_node::{health, hooks, link, registry, sink};

#[derive(Parser)]
#[command(
    name = "rc2s2",
    version,
    about = "rc2s2 node: observe and gate agent sessions on this machine"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    #[command(subcommand)]
    Node(Node),
}

#[derive(Subcommand)]
enum Node {
    /// Run the daemon. Until the hub link exists, everything it observes goes to stdout as
    /// JSON lines.
    Serve(Serve),
    /// Print the `hooks` block for ~/.claude/settings.json.
    Hooks {
        #[arg(long, default_value = "127.0.0.1:7777")]
        hook_addr: SocketAddr,
        /// Seconds Claude waits for a hook before giving up on it.
        #[arg(long, default_value_t = 600)]
        timeout: u32,
    },
}

#[derive(Args)]
struct Serve {
    #[arg(long, default_value = "127.0.0.1:7777")]
    hook_addr: SocketAddr,
    /// Claude Code's config directory: `sessions/` and `projects/` live here.
    #[arg(long, env = "CLAUDE_CONFIG_DIR")]
    claude_dir: Option<PathBuf>,
    /// Claude Code's global state file, which holds the cached usage utilization. Defaults to
    /// `.claude.json` in the home directory, or inside `--claude-dir` when that is given.
    #[arg(long)]
    claude_json: Option<PathBuf>,
    #[arg(long, default_value_t = 30)]
    health_secs: u64,
    #[arg(long, default_value_t = 2)]
    registry_secs: u64,
    #[arg(long, default_value_t = 500)]
    transcript_poll_ms: u64,
    /// The hub's node service, `http://host:port`. Without it everything goes to stdout as
    /// JSON lines.
    #[arg(long, requires = "token_file")]
    hub: Option<String>,
    /// A file holding this node's pre-shared hub token.
    #[arg(long)]
    token_file: Option<PathBuf>,
    /// The name this node registers under; the hub keys nodes by token, not by name.
    #[arg(long)]
    name: Option<String>,
    /// Unacked events kept while the hub is unreachable.
    #[arg(long, default_value_t = 200_000)]
    buffer_events: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();
    match Cli::parse().command {
        Command::Node(Node::Hooks { hook_addr, timeout }) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&hooks::settings(hook_addr, timeout))?
            );
            Ok(())
        }
        Command::Node(Node::Serve(args)) => serve(args).await,
    }
}

async fn serve(args: Serve) -> Result<()> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .context("HOME is not set")?;
    let claude_json = match (&args.claude_json, &args.claude_dir) {
        (Some(path), _) => path.clone(),
        (None, Some(dir)) => dir.join(".claude.json"),
        (None, None) => home.join(".claude.json"),
    };
    let claude_dir = args.claude_dir.unwrap_or_else(|| home.join(".claude"));

    let (outbox, rx) = Outbox::new(1024);
    let sessions = Arc::new(Sessions::new(
        claude_dir.clone(),
        outbox.clone(),
        Duration::from_millis(args.transcript_poll_ms),
    ));
    let drain = match (args.hub, args.token_file) {
        (Some(hub), Some(token_file)) => {
            let token = std::fs::read_to_string(&token_file)
                .with_context(|| format!("reading {}", token_file.display()))?
                .trim()
                .to_string();
            let name = args
                .name
                .or_else(sysinfo::System::host_name)
                .unwrap_or_else(|| "node".to_string());
            let cfg = link::Config {
                hub,
                token,
                name,
                buffer_cap: args.buffer_events,
            };
            tokio::spawn(link::run(cfg, rx, sessions.clone()))
        }
        _ => tokio::spawn(sink::stdout(rx)),
    };

    let listener = tokio::net::TcpListener::bind(args.hook_addr)
        .await
        .with_context(|| format!("binding the hook endpoint on {}", args.hook_addr))?;
    tracing::info!(addr = %args.hook_addr, claude_dir = %claude_dir.display(), "node serving");
    let app = hooks::router(outbox.clone(), sessions.clone());
    tokio::spawn(async move { axum::serve(listener, app).await });

    let registry_dir = claude_dir.join("sessions");
    let scan = sessions.clone();
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(args.registry_secs));
        loop {
            tick.tick().await;
            let dir = registry_dir.clone();
            let live = tokio::task::spawn_blocking(move || registry::live(&dir))
                .await
                .unwrap_or_default();
            scan.apply_registry(&live).await;
        }
    });

    let health_outbox = outbox.clone();
    tokio::spawn(async move {
        let mut sampler = health::Sampler::new(home, claude_json);
        let mut tick = tokio::time::interval(Duration::from_secs(args.health_secs));
        // The first tick is immediate; skip it so CPU usage has an interval to measure.
        tick.tick().await;
        loop {
            tick.tick().await;
            let sample = tokio::task::block_in_place(|| sampler.sample());
            health_outbox.health(sample).await;
        }
    });

    drop(outbox);
    tokio::select! {
        _ = tokio::signal::ctrl_c() => tracing::info!("stopping"),
        _ = drain => {}
    }
    Ok(())
}
