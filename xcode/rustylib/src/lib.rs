use std::str::FromStr;

use radicle::prelude::{NodeId, RepoId};

uniffi::setup_scaffolding!();

#[uniffi::export]
pub fn rust_hello() -> String {
    "Hello from Rust!".to_string()
}

#[uniffi::export]
pub fn rust_add(a: u32, b: u32) -> u32 {
    a + b
}

#[uniffi::export]
pub fn heartwood_version() -> String {
    option_env!("RADICLE_VERSION")
        .unwrap_or("unknown")
        .to_string()
}

#[uniffi::export]
pub fn heartwood_commit() -> String {
    option_env!("GIT_HEAD").unwrap_or("unknown").to_string()
}

#[uniffi::export]
pub fn normalize_repo_id(input: String) -> Option<String> {
    RepoId::from_str(&input)
        .ok()
        .map(|repo_id| repo_id.to_string())
}

#[uniffi::export]
pub fn normalize_node_id(input: String) -> Option<String> {
    NodeId::from_str(&input)
        .ok()
        .map(|node_id| node_id.to_string())
}
