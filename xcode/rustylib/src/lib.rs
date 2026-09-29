use std::str::FromStr;

use radicle::storage::ReadStorage;

mod bridge;

pub use bridge::{
    HeartwoodError, HeartwoodNodeInfo, HeartwoodPaths, HeartwoodProjectInfo,
    HeartwoodRepositoryInfo,
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
            matches!(result, Err(bridge::HeartwoodError::InvalidAlias(_))),
            "empty alias should be rejected"
        );
    }

    #[test]
    fn test_set_relay_invalid() {
        let result = heartwood_set_relay("fast".to_string());
        assert!(
            matches!(result, Err(bridge::HeartwoodError::InvalidRelay(_))),
            "invalid relay mode should be rejected"
        );
    }

    #[test]
    fn test_set_network_invalid() {
        let result = heartwood_set_network("prod".to_string());
        assert!(
            matches!(result, Err(bridge::HeartwoodError::InvalidNetwork(_))),
            "invalid network mode should be rejected"
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
}
