use std::fmt;

use serde::{Deserialize, Serialize};

use radicle::git::Oid;
use radicle::prelude::RepoId;

use crate::msg::{Revision, RunId, RunResult};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Run {
    broker_run_id: RunId,
    adapter_run_id: Option<RunId>,
    adapter_info_url: Option<String>,
    repo_id: RepoId,
    #[serde(alias = "repo_alias")]
    repo_name: String,
    timestamp: String,
    whence: Whence,
    state: RunState,
    result: Option<RunResult>,
}

impl Run {
    /// Create a new run.
    pub fn new(repo_id: RepoId, name: &str, whence: Whence, timestamp: String) -> Self {
        Self {
            broker_run_id: RunId::default(),
            adapter_run_id: None,
            adapter_info_url: None,
            repo_id,
            repo_name: name.into(),
            timestamp,
            whence,
            state: RunState::Triggered,
            result: None,
        }
    }

    /// Return the repo alias.
    pub fn repo_alias(&self) -> &str {
        &self.repo_name
    }

    /// Return the repo id.
    pub fn repo_id(&self) -> RepoId {
        self.repo_id
    }

    /// Return timestamp of run.
    pub fn timestamp(&self) -> &str {
        &self.timestamp
    }

    /// Return where the commit came from.
    pub fn whence(&self) -> &Whence {
        &self.whence
    }

    /// Return the run id assigned by the CI broker itself. This always exists.
    pub fn broker_run_id(&self) -> &RunId {
        &self.broker_run_id
    }

    /// Set the run id assigned by the adapter.
    pub fn set_adapter_run_id(&mut self, run_id: RunId) {
        assert!(self.adapter_run_id.is_none());
        self.adapter_run_id = Some(run_id);
    }

    /// Return the run id assigned by the adapter, if any.
    pub fn adapter_run_id(&self) -> Option<&RunId> {
        self.adapter_run_id.as_ref()
    }

    /// Set the info URL reported by the adapter.
    pub fn set_adapter_info_url(&mut self, info_url: &str) {
        self.adapter_info_url = Some(info_url.into());
    }

    /// Return the info URL if any.
    pub fn adapter_info_url(&self) -> Option<&str> {
        self.adapter_info_url.as_deref()
    }

    /// Return the state of the CI run.
    pub fn state(&self) -> RunState {
        self.state
    }

    /// Set the state of the run.
    pub fn set_state(&mut self, state: RunState) {
        self.state = state;
    }

    /// Set the result of the CI run.
    pub fn set_result(&mut self, result: RunResult) {
        self.result = Some(result);
    }

    /// Unset result of the CI run.
    pub fn unset_result(&mut self) {
        self.result = None;
    }

    /// Return the result of the CI run.
    pub fn result(&self) -> Option<&RunResult> {
        self.result.as_ref()
    }
}

/// State of CI run.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunState {
    /// Run has been triggered, but has not yet been started.
    Triggered,

    /// Run is currently running.
    Running,

    /// Run has finished. It may or may not have succeeded.
    Finished,
}

impl fmt::Display for RunState {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let s = match self {
            Self::Finished => "finished",
            Self::Running => "running",
            Self::Triggered => "triggered",
        };
        write!(f, "{}", s)
    }
}

/// Where did the commit come that CI is run for?
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum Whence {
    Branch {
        name: String,
        commit: Oid,
        who: Option<String>,
    },
    Patch {
        patch: Oid,
        commit: Oid,
        revision: Option<Revision>,
        who: Option<String>,
    },
}

impl Whence {
    pub fn branch(name: &str, commit: Oid, who: Option<&str>) -> Self {
        Self::Branch {
            name: name.into(),
            commit,
            who: who.map(|s| s.to_string()),
        }
    }

    pub fn patch(patch: Oid, commit: Oid, revision: Revision, who: Option<&str>) -> Self {
        Self::Patch {
            patch,
            commit,
            revision: Some(revision),
            who: who.map(|s| s.to_string()),
        }
    }
}

impl Whence {
    pub fn who(&self) -> Option<&str> {
        match self {
            Self::Branch {
                name: _,
                commit: _,
                who,
            } => who.as_ref().map(|x| x.as_str()),
            Self::Patch {
                patch: _,
                commit: _,
                revision: _,
                who,
            } => who.as_ref().map(|x| x.as_str()),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn serialize_run_state() -> anyhow::Result<()> {
        let s = serde_json::to_string(&RunState::Finished)?;
        assert_eq!(s, r#""finished""#);
        Ok(())
    }
}
