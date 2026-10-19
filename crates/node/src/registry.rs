//! Claude Code's session registry: `<claude dir>/sessions/<pid>.json`, one record per
//! interactive session (EXP-0005). Records outlive their process, so each is checked against
//! a live pid before it counts.

use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub pid: u32,
    pub session_id: String,
    pub cwd: String,
    #[serde(default)]
    pub started_at: i64,
    #[serde(default)]
    pub status: String,
}

/// Every record in the registry whose process is alive. Unreadable records are skipped.
pub fn live(sessions_dir: &Path) -> Vec<Record> {
    let Ok(entries) = std::fs::read_dir(sessions_dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .filter_map(|p| std::fs::read(&p).ok())
        .filter_map(|bytes| serde_json::from_slice::<Record>(&bytes).ok())
        .filter(|r| alive(r.pid))
        .collect()
}

fn alive(pid: u32) -> bool {
    let Ok(pid) = libc::pid_t::try_from(pid) else {
        return false;
    };
    // Signal 0 checks for existence; EPERM means it exists under another user.
    let ok = unsafe { libc::kill(pid, 0) } == 0;
    ok || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_live_records_and_skips_dead_and_unreadable_ones() {
        let dir = tempfile::tempdir().unwrap();
        let me = std::process::id();
        let record = |pid: u32| {
            format!(
                r#"{{"pid":{pid},"sessionId":"s-{pid}","cwd":"/w","startedAt":1,"status":"busy",
                    "kind":"interactive","messagingSocketPath":"/tmp/cc-socks/{pid}.sock"}}"#
            )
        };
        std::fs::write(dir.path().join(format!("{me}.json")), record(me)).unwrap();
        // pid_max is far below this on macOS and Linux, so it is never alive.
        std::fs::write(dir.path().join("99999999.json"), record(99_999_999)).unwrap();
        std::fs::write(dir.path().join("1.json"), "{not json").unwrap();
        std::fs::write(
            dir.path().join(format!("{me}.{}.key", "0".repeat(64))),
            "secret",
        )
        .unwrap();

        let live = live(dir.path());
        assert_eq!(live.len(), 1);
        assert_eq!(live[0].session_id, format!("s-{me}"));
        assert_eq!(live[0].status, "busy");
    }
}
