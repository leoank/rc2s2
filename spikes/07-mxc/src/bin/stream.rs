//! EXP-20261107-413q82 spike: a long-lived process under mxc's seatbelt backend with its
//! stdio relayed, the way the node's spawner drives a stream-json session.
//!
//! Usage: mxc-stream <policy.json> -- <cmd> [args...]
//!
//! The policy file is the same shape as `mxc-spike`'s. Lines on this process's stdin go to
//! the child's stdin, except `__KILL__`, which kills the child through `MxcProcess::kill`;
//! end of input closes the child's stdin. The child's stdout and stderr are relayed. The
//! child's pid and its exit are reported on stderr as `mxc-stream: ...` lines.

use std::collections::BTreeMap;
use std::io::{BufRead, Write};
use std::sync::{Arc, Mutex};

use mxc_sdk::v1::configs::SeatbeltConfig;
use mxc_sdk::v1::policy::{
    FilesystemPolicy, NetworkAction, NetworkEgressPolicy, NetworkIngressPolicy, NetworkPolicy,
};
use mxc_sdk::v1::{self, ContainerRequest, Containment};
use serde::Deserialize;

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Policy {
    #[serde(default)]
    readwrite: Vec<String>,
    #[serde(default)]
    readonly: Vec<String>,
    #[serde(default)]
    denied: Vec<String>,
    egress: Option<String>,
    ingress: Option<String>,
    host_loopback: Option<String>,
    #[serde(default)]
    keychain: bool,
    #[serde(default)]
    extra_mach_lookups: Vec<String>,
    #[serde(default)]
    env: BTreeMap<String, String>,
    cwd: Option<String>,
    timeout_ms: Option<u32>,
}

fn action(s: &Option<String>) -> Option<NetworkAction> {
    match s.as_deref() {
        None => None,
        Some("allow") => Some(NetworkAction::Allow),
        Some("deny") => Some(NetworkAction::Deny),
        Some(other) => panic!("bad network action {other:?}"),
    }
}

fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let sep = args.iter().position(|a| a == "--").expect("usage: mxc-stream <policy.json> -- cmd...");
    // `exec`, so the child mxc reports and kills is the command, not the shell around it.
    let cmd = format!(
        "exec {}",
        args[sep + 1..].iter().map(|a| shell_quote(a)).collect::<Vec<_>>().join(" ")
    );
    let policy: Policy =
        serde_json::from_str(&std::fs::read_to_string(&args[0]).expect("read policy")).expect("parse policy");

    let mut seatbelt = SeatbeltConfig::default();
    seatbelt.keychain_access = policy.keychain;
    seatbelt.extra_mach_lookups = policy.extra_mach_lookups;
    let request = ContainerRequest {
        containment: Containment::Seatbelt(seatbelt),
        filesystem: Some(FilesystemPolicy {
            readwrite_paths: policy.readwrite,
            readonly_paths: policy.readonly,
            denied_paths: policy.denied,
            ..Default::default()
        }),
        network: Some(NetworkPolicy {
            egress: Some(NetworkEgressPolicy { default: action(&policy.egress), ..Default::default() }),
            ingress: Some(NetworkIngressPolicy {
                default: action(&policy.ingress),
                host_loopback: action(&policy.host_loopback),
            }),
            ..Default::default()
        }),
        environment: if policy.env.is_empty() { None } else { Some(policy.env.into_iter().collect()) },
        inherit_default_environment: Some(false),
        working_directory: policy.cwd,
        timeout_ms: policy.timeout_ms,
        ..ContainerRequest::new(cmd)
    };

    let mut child = match v1::spawn(request, Default::default()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("mxc-stream: spawn failed: {e:?}");
            std::process::exit(2);
        }
    };
    eprintln!("mxc-stream: pid {}", child.id());
    for w in child.warnings() {
        eprintln!("mxc-stream: warning {w}");
    }
    let mut stdin = child.take_stdin().expect("no stdin from mxc");
    let mut stdout = child.take_stdout().expect("no stdout from mxc");
    let mut stderr = child.take_stderr().expect("no stderr from mxc");
    let out = std::thread::spawn(move || std::io::copy(&mut stdout, &mut std::io::stdout()));
    let err = std::thread::spawn(move || std::io::copy(&mut stderr, &mut std::io::stderr()));

    let child = Arc::new(Mutex::new(child));
    let killer = child.clone();
    std::thread::spawn(move || {
        for line in std::io::stdin().lock().lines() {
            let Ok(line) = line else { break };
            if line == "__KILL__" {
                eprintln!("mxc-stream: kill: {:?}", killer.lock().unwrap().kill());
                continue;
            }
            if stdin.write_all(line.as_bytes()).and_then(|_| stdin.write_all(b"\n")).and_then(|_| stdin.flush()).is_err() {
                break;
            }
        }
        eprintln!("mxc-stream: input closed");
        drop(stdin);
    });

    // Poll rather than block in wait(), so the input thread can take the lock to kill.
    let code = loop {
        if let Some(code) = child.lock().unwrap().try_wait().expect("try_wait") {
            break code;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    };
    let _ = out.join();
    let _ = err.join();
    eprintln!("mxc-stream: exited {code}");
    std::process::exit(if code < 0 { 128 } else { code });
}
