use std::str::FromStr;

use radicle::identity::Doc;
use radicle::identity::doc::{GetPayload, PayloadId};
use radicle::identity::project::Project;
use radicle::node::{self, Alias, NodeId};
use radicle::prelude::RepoId;
use radicle::profile;
use radicle::storage::{ReadStorage, RepositoryInfo, SignedRefsInfo};

uniffi::setup_scaffolding!();

#[derive(Debug, uniffi::Record)]
pub struct HeartwoodPaths {
    pub home: String,
    pub storage: String,
    pub config: String,
    pub keys: String,
    pub node: String,
}

#[derive(Debug, uniffi::Record)]
pub struct HeartwoodProjectInfo {
    pub name: String,
    pub description: String,
    pub default_branch: String,
}

#[derive(Debug, uniffi::Record)]
pub struct HeartwoodRepositoryInfo {
    pub rid: String,
    pub head: Option<String>,
    pub project: Option<HeartwoodProjectInfo>,
    pub delegates: Vec<String>,
    pub threshold: u32,
    pub visibility: String,
    pub refs_state: String,
    pub synced_at: Option<String>,
}

#[derive(Debug, uniffi::Record)]
pub struct HeartwoodNodeInfo {
    pub alias: String,
    pub node_id: String,
    pub user_agent: String,
    pub network: String,
    pub relay: String,
    pub external_addresses: Vec<String>,
    pub connect_addresses: Vec<String>,
    pub paths: HeartwoodPaths,
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum HeartwoodError {
    #[error("profile error: {0}")]
    Profile(String),
    #[error("storage error: {0}")]
    Storage(String),
    #[error("invalid repo id: {0}")]
    InvalidRepoId(String),
}

fn load_profile() -> Result<profile::Profile, HeartwoodError> {
    profile::Profile::load().map_err(|err| HeartwoodError::Profile(err.to_string()))
}

fn profile_paths(profile: &profile::Profile) -> HeartwoodPaths {
    HeartwoodPaths {
        home: profile.home().path().display().to_string(),
        storage: profile.home().storage().display().to_string(),
        config: profile.home().config().display().to_string(),
        keys: profile.home().keys().display().to_string(),
        node: profile.home().node().display().to_string(),
    }
}

fn project_info(doc: &Doc) -> Option<HeartwoodProjectInfo> {
    let payload = doc.get_payload(PayloadId::project())?;
    let project: Project = serde_json::from_value(payload.clone().into_inner()).ok()?;
    Some(HeartwoodProjectInfo {
        name: project.name().to_owned(),
        description: project.description().to_owned(),
        default_branch: project.default_branch().to_owned().to_string(),
    })
}

fn refs_state(info: &SignedRefsInfo) -> String {
    match info {
        SignedRefsInfo::None => "none",
        SignedRefsInfo::Some(_) => "present",
        SignedRefsInfo::NeedsMigration => "needsMigration",
    }
    .to_string()
}

fn repository_info(repo: RepositoryInfo) -> HeartwoodRepositoryInfo {
    let project = project_info(&repo.doc);
    let delegates = repo
        .doc
        .delegates()
        .iter()
        .map(|did| did.to_string())
        .collect::<Vec<_>>();

    HeartwoodRepositoryInfo {
        rid: repo.rid.to_string(),
        head: repo.head.map(|head| head.to_string()),
        project,
        delegates,
        threshold: repo.doc.threshold() as u32,
        visibility: if repo.doc.is_public() {
            "public".to_string()
        } else {
            "private".to_string()
        },
        refs_state: refs_state(&repo.refs),
        synced_at: repo
            .synced_at
            .map(|synced| format!("{} @ {}", synced.oid, synced.timestamp)),
    }
}

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
pub fn heartwood_answer() -> u32 {
    42
}

#[uniffi::export]
pub fn heartwood_ping() -> String {
    "ping".to_string()
}

#[uniffi::export]
pub fn heartwood_paths() -> Result<HeartwoodPaths, HeartwoodError> {
    let profile = load_profile()?;
    Ok(profile_paths(&profile))
}

#[uniffi::export]
pub fn heartwood_node_info() -> Result<HeartwoodNodeInfo, HeartwoodError> {
    let profile = load_profile()?;
    let config = &profile.config.node;
    let connect_addresses = config
        .connect
        .iter()
        .map(|addr| format!("{addr:?}"))
        .collect::<Vec<_>>();
    let external_addresses = config
        .external_addresses
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    Ok(HeartwoodNodeInfo {
        alias: profile.config.alias().to_string(),
        node_id: profile.id().to_string(),
        user_agent: config.user_agent().to_string(),
        network: match config.network {
            node::config::Network::Main => "main".to_string(),
            node::config::Network::Test => "test".to_string(),
        },
        relay: match config.relay {
            node::config::Relay::Always => "always".to_string(),
            node::config::Relay::Never => "never".to_string(),
            node::config::Relay::Auto => "auto".to_string(),
        },
        external_addresses,
        connect_addresses,
        paths: profile_paths(&profile),
    })
}

#[uniffi::export]
pub fn heartwood_repository_list() -> Result<Vec<HeartwoodRepositoryInfo>, HeartwoodError> {
    let profile = load_profile()?;
    let repos = profile
        .storage
        .repositories()
        .map_err(|err| HeartwoodError::Storage(err.to_string()))?
        .into_iter()
        .map(repository_info)
        .collect::<Vec<_>>();
    Ok(repos)
}

#[uniffi::export]
pub fn heartwood_repository(rid: String) -> Result<Option<HeartwoodRepositoryInfo>, HeartwoodError> {
    let profile = load_profile()?;
    let rid = RepoId::from_str(&rid).map_err(|err| HeartwoodError::InvalidRepoId(err.to_string()))?;
    let repo = profile
        .storage
        .repositories_by_id(std::iter::once(&rid))
        .next()
        .transpose()
        .map_err(|err| HeartwoodError::Storage(err.to_string()))?;
    Ok(repo.map(repository_info))
}

#[uniffi::export]
pub fn normalize_repo_id(input: String) -> Option<String> {
    RepoId::from_str(&input).ok().map(|repo_id| repo_id.to_string())
}

#[uniffi::export]
pub fn normalize_node_id(input: String) -> Option<String> {
    NodeId::from_str(&input).ok().map(|node_id| node_id.to_string())
}

#[uniffi::export]
pub fn normalize_alias(input: String) -> Option<String> {
    Alias::from_str(&input).ok().map(|alias| alias.to_string())
}
