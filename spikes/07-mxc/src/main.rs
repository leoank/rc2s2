//! EXP-0007 spike: run one command under mxc's seatbelt backend.
//!
//! Usage: mxc-spike <policy.json> -- <cmd> [args...]
//!
//! The policy file is this spike's own shape (see `Policy`), mapped onto
//! `mxc_sdk::v1::ContainerRequest`, because the SDK request type has no serde
//! derive.

use std::collections::BTreeMap;
use std::io::Write;

use mxc_sdk::v1::configs::SeatbeltConfig;
use mxc_sdk::v1::policy::{
    FilesystemPolicy, NetworkAction, NetworkEgressPolicy, NetworkIngressPolicy, NetworkPolicy,
};
use mxc_sdk::v1::{self, ContainerRequest, Containment, WaitResult};
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
    /// "allow" | "deny" (default deny)
    egress: Option<String>,
    /// "allow" | "deny" (default deny)
    ingress: Option<String>,
    /// "allow" | "deny" (default deny)
    host_loopback: Option<String>,
    #[serde(default)]
    keychain: bool,
    /// Extra Mach service global names (SeatbeltConfig::extra_mach_lookups).
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
    let sep = args.iter().position(|a| a == "--").expect("usage: mxc-spike <policy.json> -- cmd...");
    let policy_path = &args[0];
    let cmd = args[sep + 1..].iter().map(|a| shell_quote(a)).collect::<Vec<_>>().join(" ");

    let policy: Policy =
        serde_json::from_str(&std::fs::read_to_string(policy_path).expect("read policy"))
            .expect("parse policy");

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
        inherit_default_environment: Some(true),
        working_directory: policy.cwd,
        timeout_ms: policy.timeout_ms.or(Some(120_000)),
        ..ContainerRequest::new(cmd)
    };

    eprintln!("mxc-spike: {}", request.command);
    let result = match v1::run(request, Default::default()) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("MXC error: {e}");
            eprintln!("MXC error (debug): {e:?}");
            std::process::exit(2);
        }
    };
    std::io::stdout().write_all(&result.stdout).unwrap();
    std::io::stderr().write_all(&result.stderr).unwrap();
    for w in &result.warnings {
        eprintln!("mxc warning: {w}");
    }
    match result.outcome {
        WaitResult::Exited(code) => {
            eprintln!("mxc-spike: exited {code}");
            std::process::exit(code);
        }
        WaitResult::TimedOut => {
            eprintln!("mxc-spike: timed out");
            std::process::exit(124);
        }
    }
}
