//! Node health, one sample per interval: CPU, memory, the disk holding the home directory,
//! network since the last sample, running claude processes, the GPU on macOS (EXP-0009),
//! and the account's usage utilization as `/usage` last cached it (EXP-0002).

use std::path::{Path, PathBuf};

use rc2s2_proto::event::v1::{HealthSample, Metric};
use serde_json::Value;
use sysinfo::{Disks, Networks, ProcessRefreshKind, ProcessesToUpdate, System};

use crate::outbox::now_ms;

/// Process names of Claude Code: the binary, or its Nix wrapper's target.
const CLAUDE_NAMES: [&str; 2] = ["claude", ".claude-wrapped"];

pub struct Sampler {
    system: System,
    disks: Disks,
    networks: Networks,
    home: PathBuf,
    claude_json: PathBuf,
}

impl Sampler {
    pub fn new(home: PathBuf, claude_json: PathBuf) -> Self {
        let mut system = System::new();
        // CPU usage is a difference between two refreshes; this is the first.
        system.refresh_cpu_usage();
        Self {
            system,
            disks: Disks::new_with_refreshed_list(),
            networks: Networks::new_with_refreshed_list(),
            home,
            claude_json,
        }
    }

    pub fn sample(&mut self) -> HealthSample {
        let mut metrics = Vec::new();
        let mut put = |name: &str, value: f64| {
            metrics.push(Metric {
                name: name.to_string(),
                value,
            })
        };

        self.system.refresh_cpu_usage();
        self.system.refresh_memory();
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing(),
        );
        put("cpu.util_pct", self.system.global_cpu_usage() as f64);
        put("cpu.load_1m", System::load_average().one);
        put("mem.used_bytes", self.system.used_memory() as f64);
        put("mem.total_bytes", self.system.total_memory() as f64);
        put("swap.used_bytes", self.system.used_swap() as f64);

        self.disks.refresh(true);
        if let Some(disk) = self
            .disks
            .iter()
            .filter(|d| self.home.starts_with(d.mount_point()))
            .max_by_key(|d| d.mount_point().as_os_str().len())
        {
            put(
                "disk.used_bytes",
                (disk.total_space() - disk.available_space()) as f64,
            );
            put("disk.total_bytes", disk.total_space() as f64);
        }

        self.networks.refresh(true);
        let (rx, tx) = self
            .networks
            .values()
            .fold((0, 0), |(r, t), n| (r + n.received(), t + n.transmitted()));
        put("net.rx_bytes", rx as f64);
        put("net.tx_bytes", tx as f64);

        let claude = self
            .system
            .processes()
            .values()
            .filter(|p| CLAUDE_NAMES.iter().any(|n| p.name() == *n))
            .count();
        put("claude.processes", claude as f64);

        for (name, value) in gpu::sample() {
            put(name, value);
        }
        for (name, value) in cached_usage(&self.claude_json, now_ms()) {
            put(name, value);
        }
        HealthSample {
            time_ms: now_ms(),
            metrics,
        }
    }
}

/// `cachedUsageUtilization` from `~/.claude.json`, with its age. Only `/usage` writes it, so
/// the age matters as much as the value; absent after `/login` or `/logout`.
fn cached_usage(claude_json: &Path, now: i64) -> Vec<(&'static str, f64)> {
    let Some(cached) = std::fs::read(claude_json)
        .ok()
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .map(|v| v["cachedUsageUtilization"].clone())
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    if let Some(fetched) = cached["fetchedAtMs"].as_i64() {
        out.push(("usage.age_s", ((now - fetched) / 1000) as f64));
    }
    for (name, window) in [
        ("usage.five_hour_pct", "five_hour"),
        ("usage.seven_day_pct", "seven_day"),
    ] {
        if let Some(pct) = cached["utilization"][window]["utilization"].as_f64() {
            out.push((name, pct));
        }
    }
    out
}

#[cfg(target_os = "macos")]
mod gpu {
    //! IOKit `IOAccelerator` `PerformanceStatistics`, no sudo, under 100 µs (EXP-0009).

    use core_foundation::base::{CFType, TCFType, kCFAllocatorDefault};
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::number::CFNumber;
    use core_foundation::string::CFString;
    use io_kit_sys::types::{io_iterator_t, io_object_t};
    use io_kit_sys::{
        IOIteratorNext, IOObjectRelease, IORegistryEntryCreateCFProperty,
        IOServiceGetMatchingServices, IOServiceMatching, kIOMasterPortDefault,
    };

    /// Highest device utilization and total memory in use across accelerators.
    pub fn sample() -> Vec<(&'static str, f64)> {
        let (mut util, mut mem, mut found) = (0i64, 0i64, false);
        unsafe {
            let mut iter: io_iterator_t = 0;
            if IOServiceGetMatchingServices(
                kIOMasterPortDefault,
                IOServiceMatching(c"IOAccelerator".as_ptr()),
                &mut iter,
            ) != 0
            {
                return Vec::new();
            }
            loop {
                let svc: io_object_t = IOIteratorNext(iter);
                if svc == 0 {
                    break;
                }
                let key = CFString::from_static_string("PerformanceStatistics");
                let raw = IORegistryEntryCreateCFProperty(
                    svc,
                    key.as_concrete_TypeRef(),
                    kCFAllocatorDefault,
                    0,
                );
                if !raw.is_null() {
                    let stats: CFDictionary<CFString, CFType> =
                        CFDictionary::wrap_under_create_rule(raw as _);
                    let get = |k: &'static str| {
                        stats
                            .find(CFString::from_static_string(k))
                            .and_then(|v| v.downcast::<CFNumber>())
                            .and_then(|n| n.to_i64())
                    };
                    if let Some(u) = get("Device Utilization %") {
                        util = util.max(u);
                        found = true;
                    }
                    mem += get("In use system memory").unwrap_or(0);
                }
                IOObjectRelease(svc);
            }
            IOObjectRelease(iter);
        }
        if !found {
            return Vec::new();
        }
        vec![
            ("gpu.util_pct", util as f64),
            ("gpu.mem_used_bytes", mem as f64),
        ]
    }
}

#[cfg(not(target_os = "macos"))]
mod gpu {
    //! NVML and the amdgpu sysfs files come later; no GPU metrics on Linux yet.
    pub fn sample() -> Vec<(&'static str, f64)> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cached_usage_reads_percent_and_age() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".claude.json");
        std::fs::write(
            &path,
            r#"{"cachedUsageUtilization":{"fetchedAtMs":1000,"utilization":
                {"five_hour":{"utilization":23.0},"seven_day":{"utilization":41.5}}}}"#,
        )
        .unwrap();
        let got = cached_usage(&path, 61_000);
        assert_eq!(
            got,
            [
                ("usage.age_s", 60.0),
                ("usage.five_hour_pct", 23.0),
                ("usage.seven_day_pct", 41.5)
            ]
        );
        assert!(cached_usage(&dir.path().join("missing"), 0).is_empty());
    }

    #[test]
    fn a_sample_has_the_core_metrics() {
        let home = std::env::temp_dir();
        let mut sampler = Sampler::new(home.clone(), home.join("no-such-claude.json"));
        let names: Vec<String> = sampler
            .sample()
            .metrics
            .into_iter()
            .map(|m| m.name)
            .collect();
        for n in [
            "cpu.util_pct",
            "mem.used_bytes",
            "mem.total_bytes",
            "net.rx_bytes",
            "claude.processes",
        ] {
            assert!(names.iter().any(|m| m == n), "{n} missing from {names:?}");
        }
    }
}
