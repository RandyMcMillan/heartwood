use std::str::FromStr;

use radicle::identity::Doc;
use radicle::identity::doc::{GetPayload, PayloadId};
use radicle::identity::project::Project;
use radicle::node::{self, Alias, AliasStore, NodeId};
use radicle::node::{Handle, routing::Store as RoutingStore};
use radicle::prelude::RepoId;
use radicle::profile;
use radicle::storage::{ReadRepository, ReadStorage, RemoteRepository, RepositoryInfo, SignedRefsInfo};

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
    pub listen_addresses: Vec<String>,
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
    let listen_addresses = config
        .listen
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
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
        listen_addresses,
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

pub fn add_listen_address(address: &str) -> Result<(), HeartwoodError> {
    let profile = load_profile()?;
    let addr = std::net::SocketAddr::from_str(address)
        .map_err(|err| HeartwoodError::InvalidAddress(err.to_string()))?;
    let mut config = profile.config.clone();
    if !config.node.listen.contains(&addr) {
        config.node.listen.push(addr);
    }
    config
        .write(profile.home().config().as_path())
        .map_err(|err| HeartwoodError::ConfigWrite(err.to_string()))?;
    Ok(())
}

pub fn remove_listen_address(address: &str) -> Result<(), HeartwoodError> {
    let profile = load_profile()?;
    let addr = std::net::SocketAddr::from_str(address)
        .map_err(|err| HeartwoodError::InvalidAddress(err.to_string()))?;
    let mut config = profile.config.clone();
    config.node.listen.retain(|a| a != &addr);
    config
        .write(profile.home().config().as_path())
        .map_err(|err| HeartwoodError::ConfigWrite(err.to_string()))?;
    Ok(())
}

#[derive(Debug, uniffi::Record)]
pub struct HeartwoodNodeStatus {
    pub running: bool,
    pub socket: String,
}

#[derive(Debug, uniffi::Record)]
pub struct HeartwoodRoutingSummary {
    pub entries: u64,
    pub seeded_repos: u64,
}

#[derive(Debug, uniffi::Record)]
pub struct HeartwoodIssueCounts {
    pub open: u64,
    pub closed: u64,
    pub total: u64,
}

#[derive(Debug, uniffi::Record)]
pub struct HeartwoodPatchCounts {
    pub open: u64,
    pub draft: u64,
    pub archived: u64,
    pub merged: u64,
    pub total: u64,
}

pub fn node_status() -> Result<HeartwoodNodeStatus, HeartwoodError> {
    let profile = load_profile()?;
    let socket = profile.socket_from_env();
    let running = radicle::Node::new(&socket).is_running();
    Ok(HeartwoodNodeStatus {
        running,
        socket: socket.display().to_string(),
    })
}

pub fn routing_summary() -> Result<HeartwoodRoutingSummary, HeartwoodError> {
    let profile = load_profile()?;
    let routing = profile
        .routing()
        .map_err(|err| HeartwoodError::Storage(err.to_string()))?;
    let entries = routing
        .len()
        .map_err(|err| HeartwoodError::Storage(err.to_string()))? as u64;

    // Count unique seeded repos by iterating entries
    let mut repos = std::collections::HashSet::new();
    if let Ok(iter) = routing.entries() {
        for (rid, _nid) in iter {
            repos.insert(rid);
        }
    }

    Ok(HeartwoodRoutingSummary {
        entries,
        seeded_repos: repos.len() as u64,
    })
}

pub fn repository_issue_counts(rid: &str) -> Result<Option<HeartwoodIssueCounts>, HeartwoodError> {
    let profile = load_profile()?;
    let rid = RepoId::from_str(rid)
        .map_err(|err| HeartwoodError::InvalidRepoId(err.to_string()))?;
    let repo = profile
        .storage
        .repository(rid)
        .map_err(|err| HeartwoodError::Storage(err.to_string()))?;

    let counts = radicle::cob::issue::Issues::open(&repo, radicle::cob::store::access::ReadOnly)
        .map_err(|err| HeartwoodError::Storage(err.to_string()))?
        .counts()
        .map_err(|err| HeartwoodError::Storage(err.to_string()))?;

    Ok(Some(HeartwoodIssueCounts {
        open: counts.open as u64,
        closed: counts.closed as u64,
        total: counts.total() as u64,
    }))
}

pub fn repository_patch_counts(rid: &str) -> Result<Option<HeartwoodPatchCounts>, HeartwoodError> {
    let profile = load_profile()?;
    let rid = RepoId::from_str(rid)
        .map_err(|err| HeartwoodError::InvalidRepoId(err.to_string()))?;
    let repo = profile
        .storage
        .repository(rid)
        .map_err(|err| HeartwoodError::Storage(err.to_string()))?;

    let counts = radicle::cob::patch::Patches::open(&repo, radicle::cob::store::access::ReadOnly)
        .map_err(|err| HeartwoodError::Storage(err.to_string()))?
        .counts()
        .map_err(|err| HeartwoodError::Storage(err.to_string()))?;

    Ok(Some(HeartwoodPatchCounts {
        open: counts.open as u64,
        draft: counts.draft as u64,
        archived: counts.archived as u64,
        merged: counts.merged as u64,
        total: counts.total() as u64,
    }))
}

#[derive(Debug, uniffi::Record)]
pub struct HeartwoodSeedPolicy {
    pub rid: String,
    pub policy: String,
    pub scope: Option<String>,
}

#[derive(Debug, uniffi::Record)]
pub struct HeartwoodFollowPolicy {
    pub nid: String,
    pub alias: Option<String>,
    pub policy: String,
}

pub fn seed_policies() -> Result<Vec<HeartwoodSeedPolicy>, HeartwoodError> {
    let profile = load_profile()?;
    let policies = profile
        .policies_mut()
        .map_err(|err| HeartwoodError::Storage(err.to_string()))?;
    let items = policies
        .seed_policies()
        .map_err(|err| HeartwoodError::Storage(err.to_string()))?
        .filter_map(|r| r.ok())
        .map(|p| HeartwoodSeedPolicy {
            rid: p.rid.to_string(),
            policy: match p.policy {
                radicle::node::policy::SeedingPolicy::Allow { .. } => "allow".to_string(),
                radicle::node::policy::SeedingPolicy::Block => "block".to_string(),
            },
            scope: p.policy.scope().map(|s| s.to_string()),
        })
        .collect::<Vec<_>>();
    Ok(items)
}

pub fn follow_policies() -> Result<Vec<HeartwoodFollowPolicy>, HeartwoodError> {
    let profile = load_profile()?;
    let policies = profile
        .policies_mut()
        .map_err(|err| HeartwoodError::Storage(err.to_string()))?;
    let items = policies
        .follow_policies()
        .map_err(|err| HeartwoodError::Storage(err.to_string()))?
        .filter_map(|r| r.ok())
        .map(|p| HeartwoodFollowPolicy {
            nid: p.nid.to_string(),
            alias: p.alias.map(|a| a.to_string()),
            policy: match p.policy {
                radicle::node::policy::Policy::Allow => "allow".to_string(),
                radicle::node::policy::Policy::Block => "block".to_string(),
            },
        })
        .collect::<Vec<_>>();
    Ok(items)
}

pub fn is_seeding(rid: &str) -> Result<bool, HeartwoodError> {
    let profile = load_profile()?;
    let rid = RepoId::from_str(rid)
        .map_err(|err| HeartwoodError::InvalidRepoId(err.to_string()))?;
    let policies = profile
        .policies_mut()
        .map_err(|err| HeartwoodError::Storage(err.to_string()))?;
    policies
        .is_seeding(&rid)
        .map_err(|err| HeartwoodError::Storage(err.to_string()))
}

pub fn is_following(nid: &str) -> Result<bool, HeartwoodError> {
    let profile = load_profile()?;
    let nid = NodeId::from_str(nid)
        .map_err(|err| HeartwoodError::InvalidAddress(err.to_string()))?;
    let policies = profile
        .policies_mut()
        .map_err(|err| HeartwoodError::Storage(err.to_string()))?;
    policies
        .is_following(&nid)
        .map_err(|err| HeartwoodError::Storage(err.to_string()))
}

#[derive(Debug, uniffi::Record)]
pub struct HeartwoodSession {
    pub nid: String,
    pub link: String,
    pub addr: String,
    pub state: String,
}

pub fn node_sessions() -> Result<Vec<HeartwoodSession>, HeartwoodError> {
    let profile = load_profile()?;
    let socket = profile.socket_from_env();
    let node = radicle::Node::new(&socket);

    if !node.is_running() {
        return Ok(Vec::new());
    }

    let sessions = node
        .sessions()
        .map_err(|err| HeartwoodError::Storage(err.to_string()))?;

    Ok(sessions
        .into_iter()
        .map(|s| HeartwoodSession {
            nid: s.nid.to_string(),
            link: s.link.to_string(),
            addr: s.addr.to_string(),
            state: s.state.to_string(),
        })
        .collect())
}

pub fn repository_seed_count(rid: &str) -> Result<u64, HeartwoodError> {
    let profile = load_profile()?;
    let rid = RepoId::from_str(rid)
        .map_err(|err| HeartwoodError::InvalidRepoId(err.to_string()))?;
    let routing = profile
        .routing()
        .map_err(|err| HeartwoodError::Storage(err.to_string()))?;
    let count = routing
        .count(&rid)
        .map_err(|err| HeartwoodError::Storage(err.to_string()))?;
    Ok(count as u64)
}

#[derive(Debug, uniffi::Record)]
pub struct HeartwoodRemote {
    pub nid: String,
    pub refs: Vec<HeartwoodRef>,
}

#[derive(Debug, uniffi::Record)]
pub struct HeartwoodRef {
    pub name: String,
    pub oid: String,
}

pub fn repository_remotes(rid: &str) -> Result<Vec<HeartwoodRemote>, HeartwoodError> {
    let profile = load_profile()?;
    let rid = RepoId::from_str(rid)
        .map_err(|err| HeartwoodError::InvalidRepoId(err.to_string()))?;
    let repo = profile
        .storage
        .repository(rid)
        .map_err(|err| HeartwoodError::Storage(err.to_string()))?;

    let remotes = RemoteRepository::remotes(&repo)
        .map_err(|err| HeartwoodError::Storage(err.to_string()))?;

    let mut result = Vec::new();
    for (nid, _remote) in remotes {
        let refs = repo
            .references_of(&nid)
            .map_err(|err| HeartwoodError::Storage(err.to_string()))?;
        let refs = refs
            .iter()
            .map(|(name, oid)| HeartwoodRef {
                name: name.to_string(),
                oid: oid.to_string(),
            })
            .collect();
        result.push(HeartwoodRemote {
            nid: nid.to_string(),
            refs,
        });
    }

    Ok(result)
}

pub fn repository_branches(rid: &str) -> Result<Vec<HeartwoodRef>, HeartwoodError> {
    let profile = load_profile()?;
    let rid = RepoId::from_str(rid)
        .map_err(|err| HeartwoodError::InvalidRepoId(err.to_string()))?;
    let repo = profile
        .storage
        .repository(rid)
        .map_err(|err| HeartwoodError::Storage(err.to_string()))?;

    let pattern = <&radicle::git::fmt::refspec::PatternStr>::try_from("refs/heads/*")
        .map_err(|err| HeartwoodError::Storage(err.to_string()))?;
    let refs = repo
        .references_glob(pattern)
        .map_err(|err| HeartwoodError::Storage(err.to_string()))?;

    Ok(refs
        .into_iter()
        .map(|(name, oid)| HeartwoodRef {
            name: name.to_string(),
            oid: oid.to_string(),
        })
        .collect())
}

#[derive(Debug, uniffi::Record)]
pub struct HeartwoodNotificationCount {
    pub rid: String,
    pub count: u64,
}

pub fn notification_count() -> Result<u64, HeartwoodError> {
    let profile = load_profile()?;
    let notifications = profile
        .notifications_mut()
        .map_err(|err| HeartwoodError::Storage(err.to_string()))?;
    let count = notifications
        .count()
        .map_err(|err| HeartwoodError::Storage(err.to_string()))?;
    Ok(count as u64)
}

pub fn notification_counts_by_repo() -> Result<Vec<HeartwoodNotificationCount>, HeartwoodError> {
    let profile = load_profile()?;
    let notifications = profile
        .notifications_mut()
        .map_err(|err| HeartwoodError::Storage(err.to_string()))?;
    let items = notifications
        .counts_by_repo()
        .map_err(|err| HeartwoodError::Storage(err.to_string()))?
        .filter_map(|r| r.ok())
        .map(|(rid, count)| HeartwoodNotificationCount {
            rid: rid.to_string(),
            count: count as u64,
        })
        .collect::<Vec<_>>();
    Ok(items)
}

pub fn alias_for_node(nid: &str) -> Result<Option<String>, HeartwoodError> {
    let profile = load_profile()?;
    let nid = NodeId::from_str(nid)
        .map_err(|err| HeartwoodError::InvalidAddress(err.to_string()))?;
    let aliases = profile.aliases();
    Ok(aliases.alias(&nid).map(|a| a.to_string()))
}

pub fn nodes_for_alias(alias: &str) -> Result<Vec<String>, HeartwoodError> {
    let profile = load_profile()?;
    let alias = Alias::from_str(alias)
        .map_err(|err| HeartwoodError::InvalidAlias(err.to_string()))?;
    let aliases = profile.aliases();
    let nodes = aliases
        .reverse_lookup(&alias)
        .into_values()
        .flat_map(|set| set.into_iter().map(|nid| nid.to_string()))
        .collect::<Vec<_>>();
    Ok(nodes)
}

#[derive(Debug, uniffi::Record)]
pub struct HeartwoodCommit {
    pub oid: String,
    pub message: String,
    pub author: String,
    pub timestamp: i64,
}

pub fn repository_log(rid: &str, limit: u32) -> Result<Vec<HeartwoodCommit>, HeartwoodError> {
    let profile = load_profile()?;
    let rid = RepoId::from_str(rid)
        .map_err(|err| HeartwoodError::InvalidRepoId(err.to_string()))?;
    let repo = profile
        .storage
        .repository(rid)
        .map_err(|err| HeartwoodError::Storage(err.to_string()))?;

    let (_refname, head) = repo.head().map_err(|err| HeartwoodError::Storage(err.to_string()))?;
    let revwalk = repo
        .revwalk(head)
        .map_err(|err| HeartwoodError::Storage(err.to_string()))?;

    let mut commits = Vec::new();
    for oid in revwalk.take(limit as usize) {
        let oid = oid.map_err(|err| HeartwoodError::Storage(err.to_string()))?;
        let commit = repo
            .commit(oid.into())
            .map_err(|err| HeartwoodError::Storage(err.to_string()))?;
        let message = commit.message().unwrap_or("").to_string();
        let author = commit.author().name().unwrap_or("").to_string();
        let timestamp = commit.time().seconds();
        commits.push(HeartwoodCommit {
            oid: oid.to_string(),
            message,
            author,
            timestamp,
        });
    }

    Ok(commits)
}
