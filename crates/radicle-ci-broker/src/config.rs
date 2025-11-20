//! Configuration for the CI broker and related programs.

use std::{
    collections::HashMap,
    fmt,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::filter::EventFilter;

const DEFAULT_STATUS_PAGE_UPDATE_INTERVAL: u64 = 10;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub default_adapter: String,
    pub adapters: HashMap<String, Adapter>,
    pub filters: Vec<EventFilter>,
    pub report_dir: Option<PathBuf>,
    pub status_update_interval_seconds: Option<u64>,
    pub db: PathBuf,
}

impl Config {
    pub fn load(filename: &Path) -> Result<Self, ConfigError> {
        let config =
            std::fs::read(filename).map_err(|e| ConfigError::ReadConfig(filename.into(), e))?;
        serde_yml::from_slice(&config).map_err(|e| ConfigError::ParseConfig(filename.into(), e))
    }

    pub fn adapter(&self, name: &str) -> Option<&Adapter> {
        self.adapters.get(name)
    }

    pub fn status_page_update_interval(&self) -> u64 {
        self.status_update_interval_seconds
            .unwrap_or(DEFAULT_STATUS_PAGE_UPDATE_INTERVAL)
    }

    pub fn db(&self) -> &Path {
        &self.db
    }

    pub fn to_json(&self) -> Result<String, ConfigError> {
        serde_json::to_string_pretty(self).map_err(ConfigError::ToJson)
    }
}

impl fmt::Debug for Adapter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Adapter {{ \n command: {:#?}, \n env: {:#?}, \n sensitive_env: {:#?} }}",
            self.command,
            self.env,
            self.sensitive_env
                .keys()
                .map(|k| (k.to_string(), "***".to_string()))
                .collect::<HashMap<String, String>>()
        )
    }
}

#[derive(Serialize, Deserialize)]
pub struct Adapter {
    pub command: PathBuf,
    pub env: HashMap<String, String>,
    pub sensitive_env: HashMap<String, String>,
}

impl Adapter {
    pub fn envs(&self) -> &HashMap<String, String> {
        &self.env
    }
    pub fn sensitive_envs(&self) -> &HashMap<String, String> {
        &self.sensitive_env
    }
}

/// All possible errors from configuration handling.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// Can't read config file.
    #[error("could not read config file {0}")]
    ReadConfig(PathBuf, #[source] std::io::Error),

    /// Can't parse config file as YAML.
    #[error("failed to parse configuration file as YAML: {0}")]
    ParseConfig(PathBuf, #[source] serde_yml::Error),

    /// Can't convert configuration into JSON.
    #[error("failed to convert configuration into JSON")]
    ToJson(#[source] serde_json::Error),
}
