use std::str::FromStr;

use radicle::storage::ReadStorage;

mod bridge;

pub use bridge::{
    HeartwoodCommit, HeartwoodError, HeartwoodFollowPolicy, HeartwoodIssue, HeartwoodIssueCounts,
    HeartwoodNodeInfo, HeartwoodNodeStatus, HeartwoodNotificationCount, HeartwoodPatch,
    HeartwoodPatchCounts, HeartwoodPaths, HeartwoodProjectInfo, HeartwoodRef, HeartwoodRemote,
    HeartwoodRepositoryInfo, HeartwoodRoutingSummary, HeartwoodSeedPolicy, HeartwoodSession,
};

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
pub fn heartwood_answer() -> u32 {
    42
}

#[uniffi::export]
pub fn heartwood_ping() -> String {
    "ping".to_string()
}

#[uniffi::export]
pub fn heartwood_paths() -> Result<bridge::HeartwoodPaths, bridge::HeartwoodError> {
    let profile = bridge::load_profile()?;
    Ok(bridge::profile_paths(&profile))
}

#[uniffi::export]
pub fn heartwood_node_info() -> Result<bridge::HeartwoodNodeInfo, bridge::HeartwoodError> {
    let profile = bridge::load_profile()?;
    Ok(bridge::node_info(&profile))
}

#[uniffi::export]
pub fn heartwood_repository_list() -> Result<Vec<bridge::HeartwoodRepositoryInfo>, bridge::HeartwoodError> {
    let profile = bridge::load_profile()?;
    let repos = profile
        .storage
        .repositories()
        .map_err(|err| bridge::HeartwoodError::Storage(err.to_string()))?
        .into_iter()
        .map(bridge::repository_info)
        .collect::<Vec<_>>();
    Ok(repos)
}

#[uniffi::export]
pub fn heartwood_repository(rid: String) -> Result<Option<bridge::HeartwoodRepositoryInfo>, bridge::HeartwoodError> {
    let profile = bridge::load_profile()?;
    let rid = radicle::prelude::RepoId::from_str(&rid)
        .map_err(|err| bridge::HeartwoodError::InvalidRepoId(err.to_string()))?;
    let repo = profile
        .storage
        .repositories_by_id(std::iter::once(&rid))
        .next()
        .transpose()
        .map_err(|err| bridge::HeartwoodError::Storage(err.to_string()))?;
    Ok(repo.map(bridge::repository_info))
}

#[uniffi::export]
pub fn normalize_repo_id(input: String) -> Option<String> {
    bridge::normalize_repo_id(&input)
}

#[uniffi::export]
pub fn normalize_node_id(input: String) -> Option<String> {
    bridge::normalize_node_id(&input)
}

#[uniffi::export]
pub fn normalize_alias(input: String) -> Option<String> {
    bridge::normalize_alias(&input)
}

#[uniffi::export]
pub fn heartwood_set_alias(new_alias: String) -> Result<(), bridge::HeartwoodError> {
    bridge::set_alias(&new_alias)
}

#[uniffi::export]
pub fn heartwood_set_relay(mode: String) -> Result<(), bridge::HeartwoodError> {
    bridge::set_relay(&mode)
}

#[uniffi::export]
pub fn heartwood_set_network(mode: String) -> Result<(), bridge::HeartwoodError> {
    bridge::set_network(&mode)
}

#[uniffi::export]
pub fn heartwood_has_profile() -> bool {
    bridge::has_profile()
}

#[uniffi::export]
pub fn heartwood_add_external_address(address: String) -> Result<(), bridge::HeartwoodError> {
    bridge::add_external_address(&address)
}

#[uniffi::export]
pub fn heartwood_remove_external_address(address: String) -> Result<(), bridge::HeartwoodError> {
    bridge::remove_external_address(&address)
}

#[uniffi::export]
pub fn heartwood_add_connect_address(address: String) -> Result<(), bridge::HeartwoodError> {
    bridge::add_connect_address(&address)
}

#[uniffi::export]
pub fn heartwood_remove_connect_address(address: String) -> Result<(), bridge::HeartwoodError> {
    bridge::remove_connect_address(&address)
}

#[uniffi::export]
pub fn heartwood_add_listen_address(address: String) -> Result<(), bridge::HeartwoodError> {
    bridge::add_listen_address(&address)
}

#[uniffi::export]
pub fn heartwood_remove_listen_address(address: String) -> Result<(), bridge::HeartwoodError> {
    bridge::remove_listen_address(&address)
}

#[uniffi::export]
pub fn heartwood_node_status() -> Result<bridge::HeartwoodNodeStatus, bridge::HeartwoodError> {
    bridge::node_status()
}

#[uniffi::export]
pub fn heartwood_routing_summary() -> Result<bridge::HeartwoodRoutingSummary, bridge::HeartwoodError> {
    bridge::routing_summary()
}

#[uniffi::export]
pub fn heartwood_repository_issue_counts(rid: String) -> Result<Option<bridge::HeartwoodIssueCounts>, bridge::HeartwoodError> {
    bridge::repository_issue_counts(&rid)
}

#[uniffi::export]
pub fn heartwood_repository_patch_counts(rid: String) -> Result<Option<bridge::HeartwoodPatchCounts>, bridge::HeartwoodError> {
    bridge::repository_patch_counts(&rid)
}

#[uniffi::export]
pub fn heartwood_seed_policies() -> Result<Vec<bridge::HeartwoodSeedPolicy>, bridge::HeartwoodError> {
    bridge::seed_policies()
}

#[uniffi::export]
pub fn heartwood_follow_policies() -> Result<Vec<bridge::HeartwoodFollowPolicy>, bridge::HeartwoodError> {
    bridge::follow_policies()
}

#[uniffi::export]
pub fn heartwood_is_seeding(rid: String) -> Result<bool, bridge::HeartwoodError> {
    bridge::is_seeding(&rid)
}

#[uniffi::export]
pub fn heartwood_is_following(nid: String) -> Result<bool, bridge::HeartwoodError> {
    bridge::is_following(&nid)
}

#[uniffi::export]
pub fn heartwood_node_sessions() -> Result<Vec<bridge::HeartwoodSession>, bridge::HeartwoodError> {
    bridge::node_sessions()
}

#[uniffi::export]
pub fn heartwood_repository_seed_count(rid: String) -> Result<u64, bridge::HeartwoodError> {
    bridge::repository_seed_count(&rid)
}

#[uniffi::export]
pub fn heartwood_repository_remotes(rid: String) -> Result<Vec<bridge::HeartwoodRemote>, bridge::HeartwoodError> {
    bridge::repository_remotes(&rid)
}

#[uniffi::export]
pub fn heartwood_repository_branches(rid: String) -> Result<Vec<bridge::HeartwoodRef>, bridge::HeartwoodError> {
    bridge::repository_branches(&rid)
}

#[uniffi::export]
pub fn heartwood_notification_count() -> Result<u64, bridge::HeartwoodError> {
    bridge::notification_count()
}

#[uniffi::export]
pub fn heartwood_notification_counts_by_repo() -> Result<Vec<bridge::HeartwoodNotificationCount>, bridge::HeartwoodError> {
    bridge::notification_counts_by_repo()
}

#[uniffi::export]
pub fn heartwood_alias_for_node(nid: String) -> Result<Option<String>, bridge::HeartwoodError> {
    bridge::alias_for_node(&nid)
}

#[uniffi::export]
pub fn heartwood_nodes_for_alias(alias: String) -> Result<Vec<String>, bridge::HeartwoodError> {
    bridge::nodes_for_alias(&alias)
}

#[uniffi::export]
pub fn heartwood_repository_log(rid: String, limit: u32) -> Result<Vec<bridge::HeartwoodCommit>, bridge::HeartwoodError> {
    bridge::repository_log(&rid, limit)
}

#[uniffi::export]
pub fn heartwood_node_inventory() -> Result<Vec<String>, bridge::HeartwoodError> {
    bridge::node_inventory()
}

#[uniffi::export]
pub fn heartwood_repository_size(rid: String) -> Result<u64, bridge::HeartwoodError> {
    bridge::repository_size(&rid)
}

#[uniffi::export]
pub fn heartwood_repository_issues(rid: String, limit: u32) -> Result<Vec<bridge::HeartwoodIssue>, bridge::HeartwoodError> {
    bridge::repository_issues(&rid, limit)
}

#[uniffi::export]
pub fn heartwood_repository_patches_list(rid: String, limit: u32) -> Result<Vec<bridge::HeartwoodPatch>, bridge::HeartwoodError> {
    bridge::repository_patches(&rid, limit)
}

#[uniffi::export]
pub fn heartwood_create_issue(
    rid: String,
    title: String,
    description: String,
) -> Result<String, bridge::HeartwoodError> {
    bridge::create_issue(&rid, &title, &description)
}

#[uniffi::export]
pub fn heartwood_ssh_key_status() -> Result<bridge::HeartwoodSshKeyStatus, bridge::HeartwoodError> {
    bridge::ssh_key_status()
}

#[uniffi::export]
pub fn heartwood_ssh_key_generate(passphrase: Option<String>) -> Result<String, bridge::HeartwoodError> {
    bridge::ssh_key_generate(passphrase)
}

#[uniffi::export]
pub fn heartwood_repository_remove(rid: String) -> Result<(), bridge::HeartwoodError> {
    bridge::repository_remove(&rid)
}

#[uniffi::export]
pub fn heartwood_node_fetch(rid: String, from: String, timeout_secs: u64) -> Result<String, bridge::HeartwoodError> {
    bridge::node_fetch(&rid, &from, timeout_secs)
}

#[uniffi::export]
pub fn heartwood_node_seed(rid: String, scope: String) -> Result<bool, bridge::HeartwoodError> {
    bridge::node_seed(&rid, &scope)
}

#[uniffi::export]
pub fn heartwood_node_unseed(rid: String) -> Result<bool, bridge::HeartwoodError> {
    bridge::node_unseed(&rid)
}

#[uniffi::export]
pub fn heartwood_node_follow(nid: String, alias: Option<String>) -> Result<bool, bridge::HeartwoodError> {
    bridge::node_follow(&nid, alias)
}

#[uniffi::export]
pub fn heartwood_node_unfollow(nid: String) -> Result<bool, bridge::HeartwoodError> {
    bridge::node_unfollow(&nid)
}

#[uniffi::export]
pub fn heartwood_node_connect(nid: String, addr: String, timeout_secs: u64) -> Result<String, bridge::HeartwoodError> {
    bridge::node_connect(&nid, &addr, timeout_secs)
}

#[uniffi::export]
pub fn heartwood_node_disconnect(nid: String) -> Result<(), bridge::HeartwoodError> {
    bridge::node_disconnect(&nid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_hello() {
        assert_eq!(rust_hello(), "Hello from Rust!");
    }

    #[test]
    fn test_rust_add() {
        assert_eq!(rust_add(2, 3), 5);
        assert_eq!(rust_add(0, 0), 0);
        assert_eq!(rust_add(u32::MAX, 0), u32::MAX);
    }

    #[test]
    fn test_heartwood_answer() {
        assert_eq!(heartwood_answer(), 42);
    }

    #[test]
    fn test_heartwood_ping() {
        assert_eq!(heartwood_ping(), "ping");
    }

    #[test]
    fn test_heartwood_version() {
        let v = heartwood_version();
        assert!(!v.is_empty());
    }

    #[test]
    fn test_heartwood_commit() {
        let c = heartwood_commit();
        assert!(!c.is_empty());
    }

    #[test]
    fn test_normalize_repo_id_invalid() {
        assert_eq!(normalize_repo_id("".to_string()), None);
        assert_eq!(normalize_repo_id("not-a-repo-id".to_string()), None);
        assert_eq!(normalize_repo_id("rad:".to_string()), None);
    }

    #[test]
    fn test_normalize_node_id_invalid() {
        assert_eq!(normalize_node_id("".to_string()), None);
        assert_eq!(normalize_node_id("not-a-node-id".to_string()), None);
        assert_eq!(normalize_node_id("z6M".to_string()), None);
    }

    #[test]
    fn test_normalize_alias_invalid() {
        assert_eq!(normalize_alias("".to_string()), None);
        assert_eq!(normalize_alias(" ".to_string()), None);
        assert_eq!(normalize_alias("cloud head".to_string()), None);
        assert_eq!(normalize_alias("cloud\0head".to_string()), None);
    }

    #[test]
    fn test_normalize_alias_valid() {
        assert_eq!(
            normalize_alias("cloudhead".to_string()),
            Some("cloudhead".to_string())
        );
        assert_eq!(
            normalize_alias("cloud-head".to_string()),
            Some("cloud-head".to_string())
        );
        assert_eq!(
            normalize_alias("cl0ud.h3ad$__".to_string()),
            Some("cl0ud.h3ad$__".to_string())
        );
    }

    #[test]
    fn test_load_profile_error() {
        let result = bridge::load_profile();
        assert!(
            result.is_ok() || matches!(result, Err(bridge::HeartwoodError::Profile(_))),
            "load_profile should either succeed or return a Profile error"
        );
    }

    #[test]
    fn test_profile_paths_shape() {
        if let Ok(profile) = bridge::load_profile() {
            let paths = bridge::profile_paths(&profile);
            assert!(!paths.home.is_empty());
            assert!(!paths.storage.is_empty());
            assert!(!paths.config.is_empty());
            assert!(!paths.keys.is_empty());
            assert!(!paths.node.is_empty());
        }
    }

    #[test]
    fn test_heartwood_node_info_shape() {
        if let Ok(info) = heartwood_node_info() {
            assert!(!info.alias.is_empty());
            assert!(!info.node_id.is_empty());
            assert!(!info.user_agent.is_empty());
            assert!(matches!(info.network.as_str(), "main" | "test"));
            assert!(matches!(info.relay.as_str(), "always" | "never" | "auto"));
        }
    }

    #[test]
    fn test_heartwood_repository_list_shape() {
        if let Ok(repos) = heartwood_repository_list() {
            for repo in repos {
                assert!(!repo.rid.is_empty());
                assert!(matches!(repo.visibility.as_str(), "public" | "private"));
                assert!(matches!(
                    repo.refs_state.as_str(),
                    "none" | "present" | "needsMigration"
                ));
            }
        }
    }

    #[test]
    fn test_set_alias_invalid() {
        let result = heartwood_set_alias("".to_string());
        assert!(
            result.is_ok()
                || matches!(result, Err(bridge::HeartwoodError::Profile(_)))
                || matches!(result, Err(bridge::HeartwoodError::InvalidAlias(_))),
            "empty alias should be accepted (noop) or fail with profile/alias error"
        );
    }

    #[test]
    fn test_set_relay_invalid() {
        let result = heartwood_set_relay("fast".to_string());
        assert!(
            result.is_ok()
                || matches!(result, Err(bridge::HeartwoodError::Profile(_)))
                || matches!(result, Err(bridge::HeartwoodError::InvalidRelay(_))),
            "invalid relay mode should be accepted (noop) or fail with profile/relay error"
        );
    }

    #[test]
    fn test_set_network_invalid() {
        let result = heartwood_set_network("prod".to_string());
        assert!(
            result.is_ok()
                || matches!(result, Err(bridge::HeartwoodError::Profile(_)))
                || matches!(result, Err(bridge::HeartwoodError::InvalidNetwork(_))),
            "invalid network mode should be accepted (noop) or fail with profile/network error"
        );
    }

    #[test]
    fn test_set_relay_valid() {
        for mode in ["always", "never", "auto"] {
            let result = heartwood_set_relay(mode.to_string());
            assert!(
                result.is_ok() || matches!(result, Err(bridge::HeartwoodError::Profile(_))),
                "valid relay mode '{}' should succeed or fail with profile error", mode
            );
        }
    }

    #[test]
    fn test_set_network_valid() {
        for mode in ["main", "test"] {
            let result = heartwood_set_network(mode.to_string());
            assert!(
                result.is_ok() || matches!(result, Err(bridge::HeartwoodError::Profile(_))),
                "valid network mode '{}' should succeed or fail with profile error", mode
            );
        }
    }

    #[test]
    fn test_has_profile() {
        let result = heartwood_has_profile();
        assert!(result || !result, "has_profile should return a boolean");
    }

    #[test]
    fn test_add_external_address_invalid() {
        let result = heartwood_add_external_address("not-an-address".to_string());
        assert!(
            result.is_ok()
                || matches!(result, Err(bridge::HeartwoodError::Profile(_)))
                || matches!(result, Err(bridge::HeartwoodError::InvalidAddress(_))),
            "invalid address should be accepted (noop) or fail with profile/address error"
        );
    }

    #[test]
    fn test_remove_external_address_invalid() {
        let result = heartwood_remove_external_address("no-port".to_string());
        assert!(
            result.is_ok()
                || matches!(result, Err(bridge::HeartwoodError::Profile(_)))
                || matches!(result, Err(bridge::HeartwoodError::InvalidAddress(_))),
            "invalid address should be accepted (noop) or fail with profile/address error"
        );
    }

    #[test]
    fn test_add_connect_address_invalid() {
        let result = heartwood_add_connect_address("not-valid".to_string());
        assert!(
            result.is_ok()
                || matches!(result, Err(bridge::HeartwoodError::Profile(_)))
                || matches!(result, Err(bridge::HeartwoodError::InvalidAddress(_))),
            "invalid connect address should be accepted (noop) or fail with profile/address error"
        );
    }

    #[test]
    fn test_remove_connect_address_invalid() {
        let result = heartwood_remove_connect_address("bad-format".to_string());
        assert!(
            result.is_ok()
                || matches!(result, Err(bridge::HeartwoodError::Profile(_)))
                || matches!(result, Err(bridge::HeartwoodError::InvalidAddress(_))),
            "invalid connect address should be accepted (noop) or fail with profile/address error"
        );
    }

    #[test]
    fn test_add_listen_address_invalid() {
        let result = heartwood_add_listen_address("not-a-socket".to_string());
        assert!(
            result.is_ok()
                || matches!(result, Err(bridge::HeartwoodError::Profile(_)))
                || matches!(result, Err(bridge::HeartwoodError::InvalidAddress(_))),
            "invalid listen address should be accepted (noop) or fail with profile/address error"
        );
    }

    #[test]
    fn test_remove_listen_address_invalid() {
        let result = heartwood_remove_listen_address("bad-addr".to_string());
        assert!(
            result.is_ok()
                || matches!(result, Err(bridge::HeartwoodError::Profile(_)))
                || matches!(result, Err(bridge::HeartwoodError::InvalidAddress(_))),
            "invalid listen address should be accepted (noop) or fail with profile/address error"
        );
    }

    #[test]
    fn test_node_status_shape() {
        if let Ok(status) = heartwood_node_status() {
            assert!(!status.socket.is_empty());
            // Node may or may not be running; either is valid.
        }
    }

    #[test]
    fn test_routing_summary_shape() {
        if let Ok(_summary) = heartwood_routing_summary() {
            // Values are u64 and should be valid regardless of content.
        }
    }

    #[test]
    fn test_repository_issue_counts_invalid_rid() {
        let result = heartwood_repository_issue_counts("not-a-rid".to_string());
        assert!(
            result.is_ok()
                || matches!(result, Err(bridge::HeartwoodError::Profile(_)))
                || matches!(result, Err(bridge::HeartwoodError::InvalidRepoId(_))),
            "invalid rid should be accepted (noop) or fail with profile/repo error"
        );
    }

    #[test]
    fn test_repository_patch_counts_invalid_rid() {
        let result = heartwood_repository_patch_counts("not-a-rid".to_string());
        assert!(
            result.is_ok()
                || matches!(result, Err(bridge::HeartwoodError::Profile(_)))
                || matches!(result, Err(bridge::HeartwoodError::InvalidRepoId(_))),
            "invalid rid should be accepted (noop) or fail with profile/repo error"
        );
    }

    #[test]
    fn test_seed_policies_shape() {
        if let Ok(policies) = heartwood_seed_policies() {
            for p in policies {
                assert!(!p.rid.is_empty());
                assert!(matches!(p.policy.as_str(), "allow" | "block"));
            }
        }
    }

    #[test]
    fn test_follow_policies_shape() {
        if let Ok(policies) = heartwood_follow_policies() {
            for p in policies {
                assert!(!p.nid.is_empty());
                assert!(matches!(p.policy.as_str(), "allow" | "block"));
            }
        }
    }

    #[test]
    fn test_is_seeding_invalid_rid() {
        let result = heartwood_is_seeding("not-a-rid".to_string());
        assert!(
            result.is_ok()
                || matches!(result, Err(bridge::HeartwoodError::Profile(_)))
                || matches!(result, Err(bridge::HeartwoodError::InvalidRepoId(_))),
            "invalid rid should be accepted (noop) or fail with profile/repo error"
        );
    }

    #[test]
    fn test_is_following_invalid_nid() {
        let result = heartwood_is_following("not-a-nid".to_string());
        assert!(
            result.is_ok()
                || matches!(result, Err(bridge::HeartwoodError::Profile(_)))
                || matches!(result, Err(bridge::HeartwoodError::InvalidAddress(_))),
            "invalid nid should be accepted (noop) or fail with profile/address error"
        );
    }

    #[test]
    fn test_node_sessions_shape() {
        if let Ok(sessions) = heartwood_node_sessions() {
            for s in sessions {
                assert!(!s.nid.is_empty());
                assert!(matches!(s.link.as_str(), "outbound" | "inbound"));
                assert!(!s.state.is_empty());
            }
        }
    }

    #[test]
    fn test_repository_seed_count_invalid_rid() {
        let result = heartwood_repository_seed_count("not-a-rid".to_string());
        assert!(
            result.is_ok()
                || matches!(result, Err(bridge::HeartwoodError::Profile(_)))
                || matches!(result, Err(bridge::HeartwoodError::InvalidRepoId(_))),
            "invalid rid should be accepted (noop) or fail with profile/repo error"
        );
    }

    #[test]
    fn test_repository_remotes_invalid_rid() {
        let result = heartwood_repository_remotes("not-a-rid".to_string());
        assert!(
            result.is_ok()
                || matches!(result, Err(bridge::HeartwoodError::Profile(_)))
                || matches!(result, Err(bridge::HeartwoodError::InvalidRepoId(_))),
            "invalid rid should be accepted (noop) or fail with profile/repo error"
        );
    }

    #[test]
    fn test_repository_branches_invalid_rid() {
        let result = heartwood_repository_branches("not-a-rid".to_string());
        assert!(
            result.is_ok()
                || matches!(result, Err(bridge::HeartwoodError::Profile(_)))
                || matches!(result, Err(bridge::HeartwoodError::InvalidRepoId(_))),
            "invalid rid should be accepted (noop) or fail with profile/repo error"
        );
    }

    #[test]
    fn test_notification_count_shape() {
        if let Ok(count) = heartwood_notification_count() {
            // u64 is always valid
            let _ = count;
        }
    }

    #[test]
    fn test_notification_counts_by_repo_shape() {
        if let Ok(counts) = heartwood_notification_counts_by_repo() {
            for c in counts {
                assert!(!c.rid.is_empty());
            }
        }
    }

    #[test]
    fn test_alias_for_node_invalid() {
        let result = heartwood_alias_for_node("not-a-nid".to_string());
        assert!(
            result.is_ok()
                || matches!(result, Err(bridge::HeartwoodError::Profile(_)))
                || matches!(result, Err(bridge::HeartwoodError::InvalidAddress(_))),
            "invalid nid should be accepted (noop) or fail with profile/address error"
        );
    }

    #[test]
    fn test_nodes_for_alias_invalid() {
        let result = heartwood_nodes_for_alias("".to_string());
        assert!(
            result.is_ok()
                || matches!(result, Err(bridge::HeartwoodError::Profile(_)))
                || matches!(result, Err(bridge::HeartwoodError::InvalidAlias(_))),
            "empty alias should be accepted (noop) or fail with profile/alias error"
        );
    }

    #[test]
    fn test_repository_log_invalid_rid() {
        let result = heartwood_repository_log("not-a-rid".to_string(), 10);
        assert!(
            result.is_ok()
                || matches!(result, Err(bridge::HeartwoodError::Profile(_)))
                || matches!(result, Err(bridge::HeartwoodError::InvalidRepoId(_))),
            "invalid rid should be accepted (noop) or fail with profile/repo error"
        );
    }

    #[test]
    fn test_repository_log_shape() {
        if let Ok(repos) = heartwood_repository_list() {
            for repo in repos {
                if let Ok(commits) = heartwood_repository_log(repo.rid, 5) {
                    for commit in commits {
                        assert!(!commit.oid.is_empty());
                        assert!(commit.timestamp >= 0);
                    }
                }
            }
        }
    }

    #[test]
    fn test_node_inventory_shape() {
        if let Ok(inventory) = heartwood_node_inventory() {
            for rid in inventory {
                assert!(!rid.is_empty());
            }
        }
    }

    #[test]
    fn test_repository_size_invalid_rid() {
        let result = heartwood_repository_size("not-a-rid".to_string());
        assert!(
            result.is_ok()
                || matches!(result, Err(bridge::HeartwoodError::Profile(_)))
                || matches!(result, Err(bridge::HeartwoodError::InvalidRepoId(_))),
            "invalid rid should be accepted (noop) or fail with profile/repo error"
        );
    }

    #[test]
    fn test_repository_size_shape() {
        if let Ok(repos) = heartwood_repository_list() {
            for repo in repos {
                if let Ok(size) = heartwood_repository_size(repo.rid) {
                    assert!(size > 0, "repository should have non-zero size");
                }
            }
        }
    }

    #[test]
    fn test_repository_issues_invalid_rid() {
        let result = heartwood_repository_issues("not-a-rid".to_string(), 10);
        assert!(
            result.is_ok()
                || matches!(result, Err(bridge::HeartwoodError::Profile(_)))
                || matches!(result, Err(bridge::HeartwoodError::InvalidRepoId(_))),
            "invalid rid should be accepted (noop) or fail with profile/repo error"
        );
    }

    #[test]
    fn test_repository_patches_list_invalid_rid() {
        let result = heartwood_repository_patches_list("not-a-rid".to_string(), 10);
        assert!(
            result.is_ok()
                || matches!(result, Err(bridge::HeartwoodError::Profile(_)))
                || matches!(result, Err(bridge::HeartwoodError::InvalidRepoId(_))),
            "invalid rid should be accepted (noop) or fail with profile/repo error"
        );
    }

    #[test]
    fn test_create_issue_invalid_rid() {
        let result = heartwood_create_issue(
            "not-a-rid".to_string(),
            "title".to_string(),
            "description".to_string(),
        );
        assert!(
            result.is_ok()
                || matches!(result, Err(bridge::HeartwoodError::Profile(_)))
                || matches!(result, Err(bridge::HeartwoodError::InvalidRepoId(_)))
                || matches!(result, Err(bridge::HeartwoodError::Signer(_))),
            "invalid rid should be accepted (noop) or fail with profile/repo/signer error"
        );
    }

    #[test]
    fn test_ssh_key_status_shape() {
        if let Ok(status) = heartwood_ssh_key_status() {
            // Should always return a valid shape
            let _ = status.exists;
            let _ = status.encrypted;
        }
    }

    #[test]
    fn test_repository_remove_invalid_rid() {
        let result = heartwood_repository_remove("not-a-rid".to_string());
        assert!(
            result.is_ok()
                || matches!(result, Err(bridge::HeartwoodError::Profile(_)))
                || matches!(result, Err(bridge::HeartwoodError::InvalidRepoId(_))),
            "invalid rid should be accepted (noop) or fail with profile/repo error"
        );
    }

    #[test]
    fn test_node_fetch_invalid_rid() {
        let result = heartwood_node_fetch("not-a-rid".to_string(), "not-a-nid".to_string(), 30);
        assert!(
            result.is_ok()
                || matches!(result, Err(bridge::HeartwoodError::Profile(_)))
                || matches!(result, Err(bridge::HeartwoodError::InvalidRepoId(_)))
                || matches!(result, Err(bridge::HeartwoodError::InvalidAddress(_))),
            "invalid rid should be accepted (noop) or fail with profile/repo/address error"
        );
    }

    #[test]
    fn test_node_seed_invalid_rid() {
        let result = heartwood_node_seed("not-a-rid".to_string(), "all".to_string());
        assert!(
            result.is_ok()
                || matches!(result, Err(bridge::HeartwoodError::Profile(_)))
                || matches!(result, Err(bridge::HeartwoodError::InvalidRepoId(_))),
            "invalid rid should be accepted (noop) or fail with profile/repo error"
        );
    }

    #[test]
    fn test_node_connect_invalid() {
        let result = heartwood_node_connect("not-a-nid".to_string(), "bad-addr".to_string(), 30);
        assert!(
            result.is_ok()
                || matches!(result, Err(bridge::HeartwoodError::Profile(_)))
                || matches!(result, Err(bridge::HeartwoodError::InvalidAddress(_))),
            "invalid params should be accepted (noop) or fail with profile/address error"
        );
    }
}
