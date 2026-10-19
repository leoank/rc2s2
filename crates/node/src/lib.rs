//! The node side of rc2s2: everything `rc2s2 node serve` runs, as a library so the hub's
//! end-to-end tests can drive a real node.

pub mod health;
pub mod hooks;
pub mod link;
pub mod outbox;
pub mod registry;
pub mod sessions;
pub mod sink;
pub mod transcript;
