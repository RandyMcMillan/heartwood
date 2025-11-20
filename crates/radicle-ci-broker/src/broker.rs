//! The business logic of the CI broker.
//!
//! This is type and module of its own to facilitate automated
//! testing.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use time::{macros::format_description, OffsetDateTime};

use radicle::prelude::RepoId;

use crate::{
    adapter::Adapter,
    db::{Db, DbError},
    logger,
    msg::{PatchEvent, PushEvent, Request},
    notif::NotificationSender,
    run::{Run, Whence},
};

/// A CI broker.
///
/// The broker gets repository change events from the local Radicle
/// node, and executes the appropriate adapter to run CI on the
/// repository.
pub struct Broker {
    default_adapter: Option<Adapter>,
    adapters: HashMap<RepoId, Adapter>,
    db: Db,
}

impl Broker {
    #[allow(clippy::result_large_err)]
    pub fn new(db_filename: &Path) -> Result<Self, BrokerError> {
        logger::broker_db(db_filename);
        Ok(Self {
            default_adapter: None,
            adapters: HashMap::new(),
            db: Db::new(db_filename)?,
        })
    }

    #[allow(clippy::result_large_err)]
    pub fn all_runs(&mut self) -> Result<Vec<Run>, BrokerError> {
        Ok(self.db.get_all_runs()?)
    }

    pub fn set_default_adapter(&mut self, adapter: &Adapter) {
        self.default_adapter = Some(adapter.clone());
    }

    pub fn default_adapter(&self) -> Option<&Adapter> {
        self.default_adapter.as_ref()
    }

    pub fn set_repository_adapter(&mut self, rid: &RepoId, adapter: &Adapter) {
        self.adapters.insert(*rid, adapter.clone());
    }

    pub fn adapter(&self, rid: &RepoId) -> Option<&Adapter> {
        self.adapters.get(rid).or(self.default_adapter.as_ref())
    }

    #[allow(clippy::result_large_err)]
    pub fn execute_ci(
        &mut self,
        trigger: &Request,
        run_notification: &NotificationSender,
    ) -> Result<Run, BrokerError> {
        logger::broker_start_run(trigger);
        let run = match trigger {
            Request::Trigger {
                common,
                push,
                patch,
            } => {
                let rid = &common.repository.id;
                if let Some(adapter) = self.adapter(rid) {
                    let whence = if let Some(PushEvent {
                        pusher,
                        before: _,
                        after,
                        branch: _,
                        commits: _,
                    }) = push
                    {
                        let who = pusher.to_string();
                        Whence::branch("push-event-has-no-branch-name", *after, Some(who.as_str()))
                    } else if let Some(PatchEvent { action: _, patch }) = patch {
                        let revision = patch
                            .revisions
                            .last()
                            .ok_or(BrokerError::NoRevisions)?
                            .clone();
                        let who = patch.author.to_string();
                        Whence::patch(patch.id, patch.after, revision, Some(who.as_str()))
                    } else {
                        panic!("neither push not patch event");
                    };

                    let mut run = Run::new(*rid, &common.repository.name, whence, now()?);
                    self.db.push_run(&run)?;

                    // We run the adapter, but if that fails, we just
                    // log the error. The `Run` value records the
                    // result of the run.
                    if let Err(e) = adapter.run(trigger, &mut run, &self.db, run_notification) {
                        logger::error("failed to run adapter or it failed to run CI", &e);
                    }

                    run
                } else {
                    return Err(BrokerError::NoAdapter(*rid));
                }
            }
        };
        logger::broker_end_run(&run);

        self.db.update_run(&run)?;

        Ok(run)
    }
}

fn now() -> Result<String, time::error::Format> {
    let fmt = format_description!("[year]-[month]-[day] [hour]:[minute]:[second]Z");
    OffsetDateTime::now_utc().format(fmt)
}

/// All possible errors from this module.
#[derive(Debug, thiserror::Error)]
pub enum BrokerError {
    /// Error formatting a time as a string.
    #[error(transparent)]
    Timeformat(#[from] time::error::Format),

    /// Error from Radicle.
    #[error(transparent)]
    RadicleProfile(#[from] radicle::profile::Error),

    /// Error from spawning a sub-process.
    #[error("failed to spawn a CI adapter sub-process: {0}")]
    SpawnAdapter(PathBuf, #[source] std::io::Error),

    /// Default adapter is not in list of adapters.
    #[error("default adapter is not in list of adapters")]
    UnknownDefaultAdapter(String),

    /// No adapter set for repository and no default adapter set.
    #[error("could not determine what adapter to use for repository {0}")]
    NoAdapter(RepoId),

    /// Request is not a trigger message.
    #[error("tried to execute CI based on a message that is not a trigger one: {0:#?}")]
    NotTrigger(Box<Request>),

    /// Could not convert repository ID from string.
    #[error("failed to understand repository id {0:?}")]
    BadRepoId(String, #[source] radicle::identity::IdError),

    /// Patch event doesn't have any revisions.
    #[error("expected at least one revision in a patch event")]
    NoRevisions,

    /// Database error.
    #[error(transparent)]
    Db(#[from] DbError),
}

#[cfg(test)]
mod test {
    use std::path::Path;
    use tempfile::tempdir;

    use super::{Adapter, Broker, RepoId};
    use crate::{
        msg::{RunId, RunResult},
        notif::NotificationChannel,
        run::RunState,
        test::{mock_adapter, trigger_request, TestResult},
    };

    fn broker(filename: &Path) -> anyhow::Result<Broker> {
        Ok(Broker::new(filename)?)
    }

    fn rid() -> anyhow::Result<RepoId> {
        const RID: &str = "rad:zwTxygwuz5LDGBq255RA2CbNGrz8";
        Ok(RepoId::from_urn(RID)?)
    }

    fn rid2() -> anyhow::Result<RepoId> {
        const RID: &str = "rad:z3gqcJUoA1n9HaHKufZs5FCSGazv5";
        Ok(RepoId::from_urn(RID)?)
    }

    #[test]
    fn has_no_adapters_initially() -> TestResult<()> {
        let tmp = tempdir()?;
        let db = tmp.path().join("db.db");
        let broker = broker(&db)?;
        let rid = rid()?;
        assert_eq!(broker.adapter(&rid), None);
        Ok(())
    }

    #[test]
    fn adds_adapter() -> TestResult<()> {
        let tmp = tempdir()?;
        let db = tmp.path().join("db.db");
        let mut broker = broker(&db)?;

        let adapter = Adapter::default();
        let rid = rid()?;
        broker.set_repository_adapter(&rid, &adapter);
        assert_eq!(broker.adapter(&rid), Some(&adapter));
        Ok(())
    }

    #[test]
    fn does_not_find_unknown_repo() -> TestResult<()> {
        let tmp = tempdir()?;
        let db = tmp.path().join("db.db");
        let mut broker = broker(&db)?;

        let adapter = Adapter::default();
        let rid = rid()?;
        let rid2 = rid2()?;
        broker.set_repository_adapter(&rid, &adapter);
        assert_eq!(broker.adapter(&rid2), None);
        Ok(())
    }

    #[test]
    fn does_not_have_a_default_adapter_initially() -> TestResult<()> {
        let tmp = tempdir()?;
        let db = tmp.path().join("db.db");
        let broker = broker(&db)?;

        assert_eq!(broker.default_adapter(), None);
        Ok(())
    }

    #[test]
    fn sets_a_default_adapter_initially() -> TestResult<()> {
        let tmp = tempdir()?;
        let db = tmp.path().join("db.db");
        let mut broker = broker(&db)?;

        let adapter = Adapter::default();
        broker.set_default_adapter(&adapter);
        assert_eq!(broker.default_adapter(), Some(&adapter));
        Ok(())
    }

    #[test]
    fn finds_default_adapter_for_unknown_repo() -> TestResult<()> {
        let tmp = tempdir()?;
        let db = tmp.path().join("db.db");
        let mut broker = broker(&db)?;

        let adapter = Adapter::default();
        broker.set_default_adapter(&adapter);

        let rid = rid()?;
        assert_eq!(broker.adapter(&rid), Some(&adapter));
        Ok(())
    }

    #[test]
    fn executes_adapter() -> TestResult<()> {
        const ADAPTER: &str = r#"#!/bin/bash
read
echo '{"response":"triggered","run_id":{"id":"xyzzy"}}'
echo '{"response":"finished","result":"success"}'
"#;

        let tmp = tempdir()?;
        let bin = tmp.path().join("adapter.sh");
        let adapter = mock_adapter(&bin, ADAPTER)?;

        let tmp = tempdir()?;
        let db = tmp.path().join("db.db");
        let mut broker = broker(&db)?;
        broker.set_default_adapter(&adapter);

        let trigger = trigger_request()?;

        let mut channel = NotificationChannel::new_run();
        let sender = channel.tx()?;
        let x = broker.execute_ci(&trigger, &sender);
        assert!(x.is_ok());
        let run = x?;
        assert_eq!(run.adapter_run_id(), Some(&RunId::from("xyzzy")));
        assert_eq!(run.state(), RunState::Finished);
        assert_eq!(run.result(), Some(&RunResult::Success));

        Ok(())
    }

    #[test]
    fn adapter_fails() -> TestResult<()> {
        const ADAPTER: &str = r#"#!/bin/bash
read
echo '{"response":"triggered","run_id":{"id":"xyzzy"}}'
echo '{"response":"finished","result":"success"}'
echo woe be me 1>&2
exit 1
"#;

        let tmp = tempdir()?;
        let bin = tmp.path().join("adapter.sh");
        let adapter = mock_adapter(&bin, ADAPTER)?;

        let tmp = tempdir()?;
        let db = tmp.path().join("db.db");
        let mut broker = broker(&db)?;
        broker.set_default_adapter(&adapter);

        let trigger = trigger_request()?;

        let mut channel = NotificationChannel::new_run();
        let sender = channel.tx()?;
        let x = broker.execute_ci(&trigger, &sender);
        assert!(x.is_ok());
        let run = x?;
        assert_eq!(run.adapter_run_id(), Some(&RunId::from("xyzzy")));
        assert_eq!(run.state(), RunState::Finished);
        assert_eq!(run.result(), Some(&RunResult::Success));

        Ok(())
    }
}
