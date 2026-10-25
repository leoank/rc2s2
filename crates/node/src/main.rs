//! `rc2s2`: the node side of rc2s2. `rc2s2 node serve` runs the daemon; `rc2s2 node hooks`
//! prints the settings block that points Claude Code's hooks at it.

use std::io::IsTerminal;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};

use rc2s2_node::outbox::Outbox;
use rc2s2_node::pending::Pending;
use rc2s2_node::sessions::Sessions;
use rc2s2_node::wrapper::{self, Wrappers};
use rc2s2_node::{health, hooks, link, registry, sink};

mod awake;
mod ctl;
mod usage;

#[derive(Parser)]
#[command(
    name = "rc2s2",
    version,
    about = "rc2s2: the node daemon that observes and gates agent sessions on this machine, and a client of the hub"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    #[command(subcommand)]
    Node(Node),
    /// Talk to the hub's control service as a client.
    Ctl(CtlArgs),
    /// Run Claude Code under a PTY the node daemon can interrupt and kill.
    Run(RunArgs),
}

#[derive(Args)]
struct RunArgs {
    /// The program to run.
    #[arg(long, default_value = "claude")]
    command: String,
    /// The node daemon's wrapper socket; the daemon's default when omitted.
    #[arg(long)]
    wrapper_socket: Option<PathBuf>,
    /// Arguments for the program, after `--`.
    #[arg(last = true)]
    args: Vec<String>,
}

#[derive(Args)]
struct CtlArgs {
    /// The hub, `http://host:port`.
    #[arg(long, env = "RC2S2_HUB")]
    hub: String,
    /// A file holding this client's hub token.
    #[arg(long, env = "RC2S2_TOKEN_FILE")]
    token_file: PathBuf,
    #[command(subcommand)]
    cmd: ctl::Ctl,
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
    /// Seconds a permission dialog of an unwrapped session waits for a decision from the hub
    /// before it falls through to the terminal. Keep it below the hooks' `--timeout`.
    #[arg(long, default_value_t = 120)]
    hold_secs: u64,
    /// The same for a session under `rc2s2 run`, where a keystroke at its terminal releases
    /// the hold at once. Keep it below the hooks' `--timeout`.
    #[arg(long, default_value_t = 540)]
    wrapped_hold_secs: u64,
    /// The unix socket `rc2s2 run` wrappers connect to.
    #[arg(long)]
    wrapper_socket: Option<PathBuf>,
    /// A glob over working directories whose sessions are not tracked at all: not listed,
    /// not tailed, their dialogs not held (ADR-0028). Repeatable.
    #[arg(long = "ignore-cwd")]
    ignore_cwd: Vec<String>,
    /// Report the account's rate-limit utilisation as health, from Anthropic's usage
    /// endpoint with the access token read from Claude Code's credential store, read only
    /// (ADR-0030). Off by default.
    #[arg(long)]
    claude_usage: bool,
    /// How often to poll the usage endpoint; after a 429 the daemon waits longer (it refused
    /// once-a-minute polls for half an hour, EXP-20261025-25f6vm).
    #[arg(long, default_value_t = 300)]
    claude_usage_secs: u64,
    /// The usage endpoint; overridden in tests.
    #[arg(long, default_value = usage::ENDPOINT, hide = true)]
    claude_usage_url: String,
    /// The keychain service name to read instead of Claude Code's own; for tests.
    #[arg(long, hide = true)]
    claude_usage_service: Option<String>,
    /// Keep the machine from idle-sleeping while a session is busy (`caffeinate -i` on
    /// macOS), reported as the health metric `node.keep_awake` (ADR-0031).
    #[arg(long)]
    keep_awake: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(std::io::stderr().is_terminal())
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
        Command::Run(args) => {
            let opts = rc2s2_node::run::Options {
                command: args.command,
                args: args.args,
                socket: args.wrapper_socket.unwrap_or_else(wrapper::default_socket),
            };
            let code = tokio::task::spawn_blocking(move || rc2s2_node::run::run(opts)).await??;
            std::process::exit(code)
        }
        Command::Ctl(args) => {
            let token = std::fs::read_to_string(&args.token_file)
                .with_context(|| format!("reading {}", args.token_file.display()))?;
            ctl::run(&args.hub, token.trim(), args.cmd).await
        }
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
    let default_claude_dir = args.claude_dir.is_none();
    let claude_dir = args.claude_dir.unwrap_or_else(|| home.join(".claude"));

    let (outbox, rx) = Outbox::new(1024);
    let ignore = args
        .ignore_cwd
        .iter()
        .map(|g| glob::Pattern::new(g).with_context(|| format!("--ignore-cwd {g}")))
        .collect::<Result<Vec<_>>>()?;
    let sessions = Arc::new(Sessions::new(
        claude_dir.clone(),
        outbox.clone(),
        Duration::from_millis(args.transcript_poll_ms),
        ignore,
    ));
    let pending = Pending::new(outbox.clone());
    let wrappers = Wrappers::new(sessions.clone(), pending.clone(), outbox.clone());
    let wrapper_socket = args
        .wrapper_socket
        .clone()
        .unwrap_or_else(wrapper::default_socket);
    let listener = Wrappers::bind(&wrapper_socket)?;
    tracing::info!(socket = %wrapper_socket.display(), "listening for wrappers");
    tokio::spawn(wrappers.clone().serve(listener));
    let mut hold = None;
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
            hold = Some(hooks::Hold {
                pending: pending.clone(),
                duration: Duration::from_secs(args.hold_secs),
                wrapped_duration: Duration::from_secs(args.wrapped_hold_secs),
            });
            tokio::spawn(link::run(cfg, rx, sessions.clone(), pending, wrappers))
        }
        _ => tokio::spawn(sink::stdout(rx)),
    };

    let listener = tokio::net::TcpListener::bind(args.hook_addr)
        .await
        .with_context(|| format!("binding the hook endpoint on {}", args.hook_addr))?;
    tracing::info!(addr = %args.hook_addr, claude_dir = %claude_dir.display(), "node serving");
    let app = hooks::router(outbox.clone(), sessions.clone(), hold);
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

    if args.claude_usage {
        let store = match &args.claude_usage_service {
            Some(service) => usage::Store::Keychain {
                service: service.clone(),
            },
            None => usage::Store::for_dir(&claude_dir, default_claude_dir),
        };
        let url = args.claude_usage_url.clone();
        let every = Duration::from_secs(args.claude_usage_secs);
        let usage_outbox = outbox.clone();
        tokio::spawn(async move {
            let client = reqwest::Client::new();
            let mut wait = every;
            loop {
                match usage::poll(&client, &store, &url).await {
                    Ok(sample) => {
                        usage_outbox.health(sample).await;
                        wait = every;
                    }
                    Err(e) => match e.downcast_ref::<usage::TooMany>() {
                        Some(usage::TooMany(after)) => {
                            wait = usage::backoff(every, wait, *after);
                            tracing::warn!(wait_s = wait.as_secs(), "usage endpoint refused, too many requests; backing off");
                        }
                        None => tracing::warn!(error = %format!("{e:#}"), "usage poll failed"),
                    },
                }
                tokio::time::sleep(wait).await;
            }
        });
    }

    let held: awake::Held = Arc::new(std::sync::atomic::AtomicBool::new(false));
    if args.keep_awake {
        tokio::spawn(awake::keep(sessions.clone(), held.clone(), Duration::from_secs(args.registry_secs.max(1))));
    }
    let keep_awake_on = args.keep_awake;

    let health_outbox = outbox.clone();
    tokio::spawn(async move {
        let mut sampler = health::Sampler::new(home, claude_json);
        let mut tick = tokio::time::interval(Duration::from_secs(args.health_secs));
        // The first tick is immediate; skip it so CPU usage has an interval to measure.
        tick.tick().await;
        loop {
            tick.tick().await;
            let mut sample = tokio::task::block_in_place(|| sampler.sample());
            if keep_awake_on {
                sample.metrics.push(rc2s2_proto::event::v1::Metric {
                    name: "node.keep_awake".into(),
                    value: if held.load(std::sync::atomic::Ordering::Relaxed) { 1.0 } else { 0.0 },
                });
            }
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
