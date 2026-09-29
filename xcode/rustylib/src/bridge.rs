use std::str::FromStr;

use radicle::identity::Doc;
use radicle::identity::doc::{GetPayload, PayloadId};
use radicle::identity::project::Project;
use radicle::node::{self, Alias, NodeId};
use radicle::prelude::RepoId;
use radicle::profile;
use radicle::storage::{RepositoryInfo, SignedRefsInfo};

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
    #[error("invalid alias: {0}")]
    InvalidAlias(String),
    #[error("invalid relay mode: {0}")]
    InvalidRelay(String),
    #[error("invalid network mode: {0}")]
    InvalidNetwork(String),
    #[error("config write error: {0}")]
    ConfigWrite(String),
    #[error("invalid address: {0}")]
    InvalidAddress(String),
}

pub fn load_profile() -> Result<profile::Profile, HeartwoodError> {
    profile::Profile::load().map_err(|err| HeartwoodError::Profile(err.to_string()))
}

pub fn has_profile() -> bool {
    profile::Profile::load().is_ok()
}

pub fn profile_paths(profile: &profile::Profile) -> HeartwoodPaths {
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

pub fn repository_info(repo: RepositoryInfo) -> HeartwoodRepositoryInfo {
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

pub fn node_info(profile: &profile::Profile) -> HeartwoodNodeInfo {
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

    HeartwoodNodeInfo {
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
        paths: profile_paths(profile),
    }
}

pub fn normalize_repo_id(input: &str) -> Option<String> {
    RepoId::from_str(input).ok().map(|repo_id| repo_id.to_string())
}

pub fn normalize_node_id(input: &str) -> Option<String> {
    NodeId::from_str(input).ok().map(|node_id| node_id.to_string())
}

pub fn normalize_alias(input: &str) -> Option<String> {
    Alias::from_str(input).ok().map(|alias| alias.to_string())
}

pub fn set_alias(new_alias: &str) -> Result<(), HeartwoodError> {
    let profile = load_profile()?;
    let alias = Alias::from_str(new_alias)
        .map_err(|err| HeartwoodError::InvalidAlias(err.to_string()))?;
    let mut config = profile.config.clone();
    config.node.alias = alias;
    config
        .write(profile.home().config().as_path())
        .map_err(|err| HeartwoodError::ConfigWrite(err.to_string()))?;
    Ok(())
}

pub fn set_relay(mode: &str) -> Result<(), HeartwoodError> {
    let profile = load_profile()?;
    let relay = match mode {
        "always" => node::config::Relay::Always,
        "never" => node::config::Relay::Never,
        "auto" => node::config::Relay::Auto,
        _ => return Err(HeartwoodError::InvalidRelay(mode.to_string())),
    };
    let mut config = profile.config.clone();
    config.node.relay = relay;
    config
        .write(profile.home().config().as_path())
        .map_err(|err| HeartwoodError::ConfigWrite(err.to_string()))?;
    Ok(())
}

pub fn set_network(mode: &str) -> Result<(), HeartwoodError> {
    let profile = load_profile()?;
    let network = match mode {
        "main" => node::config::Network::Main,
        "test" => node::config::Network::Test,
        _ => return Err(HeartwoodError::InvalidNetwork(mode.to_string())),
    };
    let mut config = profile.config.clone();
    config.node.network = network;
    config
        .write(profile.home().config().as_path())
        .map_err(|err| HeartwoodError::ConfigWrite(err.to_string()))?;
    Ok(())
}

pub fn add_external_address(address: &str) -> Result<(), HeartwoodError> {
    let profile = load_profile()?;
    let addr = node::Address::from_str(address)
        .map_err(|err| HeartwoodError::InvalidAddress(err.to_string()))?;
    let mut config = profile.config.clone();
    if !config.node.external_addresses.contains(&addr) {
        config.node.external_addresses.push(addr);
    }
    config
        .write(profile.home().config().as_path())
        .map_err(|err| HeartwoodError::ConfigWrite(err.to_string()))?;
    Ok(())
}

pub fn remove_external_address(address: &str) -> Result<(), HeartwoodError> {
    let profile = load_profile()?;
    let addr = node::Address::from_str(address)
        .map_err(|err| HeartwoodError::InvalidAddress(err.to_string()))?;
    let mut config = profile.config.clone();
    config.node.external_addresses.retain(|a| a != &addr);
    config
        .write(profile.home().config().as_path())
        .map_err(|err| HeartwoodError::ConfigWrite(err.to_string()))?;
    Ok(())
}

fn parse_connect_address(input: &str) -> Result<node::config::ConnectAddress, HeartwoodError> {
    let (nid, addr) = input
        .rsplit_once('@')
        .ok_or_else(|| HeartwoodError::InvalidAddress("expected nodeId@host:port".to_string()))?;
    let nid = NodeId::from_str(nid)
        .map_err(|err| HeartwoodError::InvalidAddress(err.to_string()))?;
    let addr = node::Address::from_str(addr)
        .map_err(|err| HeartwoodError::InvalidAddress(err.to_string()))?;
    Ok(node::config::ConnectAddress::from((nid, addr)))
}

pub fn add_connect_address(input: &str) -> Result<(), HeartwoodError> {
    let profile = load_profile()?;
    let connect = parse_connect_address(input)?;
    let mut config = profile.config.clone();
    if !config.node.connect.contains(&connect) {
        config.node.connect.insert(connect);
    }
    config
        .write(profile.home().config().as_path())
        .map_err(|err| HeartwoodError::ConfigWrite(err.to_string()))?;
    Ok(())
}

pub fn remove_connect_address(input: &str) -> Result<(), HeartwoodError> {
    let profile = load_profile()?;
    let connect = parse_connect_address(input)?;
    let mut config = profile.config.clone();
    config.node.connect.shift_remove(&connect);
    config
        .write(profile.home().config().as_path())
        .map_err(|err| HeartwoodError::ConfigWrite(err.to_string()))?;
    Ok(())
}
