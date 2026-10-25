//! Keeps the machine from idle-sleeping while a session is busy (ADR-0031): on macOS a
//! `caffeinate -i` child is held while any known session is busy and dropped when none is,
//! so a laptop whose screen turned off still finishes the turn and keeps its hub link. The
//! state goes out as the health metric `node.keep_awake` (1 while held). Off unless
//! `--keep-awake`; on other platforms the flag only reports 0.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use rc2s2_proto::event::v1::SessionStatus;

use rc2s2_node::sessions::Sessions;

/// Whether the hold is on, shared with the health sampler.
pub type Held = Arc<AtomicBool>;

/// Watches the sessions and holds or releases the assertion; runs until the node exits.
pub async fn keep(sessions: Arc<Sessions>, held: Held, every: Duration) {
    let mut child: Option<tokio::process::Child> = None;
    let mut tick = tokio::time::interval(every);
    loop {
        tick.tick().await;
        let busy = sessions
            .snapshot()
            .await
            .iter()
            .any(|s| s.ended_ms.is_none() && s.status() == SessionStatus::Busy);
        match (busy, child.as_mut()) {
            (true, None) => {
                if cfg!(target_os = "macos") {
                    // The assertion dies with the node: an orphaned caffeinate would hold
                    // the machine up for good.
                    match tokio::process::Command::new("/usr/bin/caffeinate")
                        .arg("-i")
                        .kill_on_drop(true)
                        .spawn()
                    {
                        Ok(c) => {
                            child = Some(c);
                            held.store(true, Ordering::Relaxed);
                            tracing::info!("keeping the machine awake: a session is busy");
                        }
                        Err(e) => tracing::warn!(error = %e, "caffeinate failed to start"),
                    }
                }
            }
            (false, Some(c)) => {
                let _ = c.start_kill();
                let _ = c.wait().await;
                child = None;
                held.store(false, Ordering::Relaxed);
                tracing::info!("letting the machine sleep: no session is busy");
            }
            (true, Some(c)) => {
                // The child died (killed by hand): take it again on the next tick.
                if let Ok(Some(_)) = c.try_wait() {
                    child = None;
                    held.store(false, Ordering::Relaxed);
                }
            }
            (false, None) => {}
        }
    }
}
