//! Run a Radicle CI adapter.
//!
//! Given an executable that conforms to the CI adapter API, execute
//! it by feeding it the "trigger" message via its stdin and reading
//! response messages from its stdout. Return the result of the run,
//! or an error if something went badly wrong. The CI run failing due
//! to something in the repository under test is expected, and not
//! considered as something going badly wrong.

use std::{
    collections::HashMap,
    ffi::OsStr,
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use crate::{
    db::{Db, DbError},
    logger,
    msg::{MessageError, Request, Response},
    notif::NotificationSender,
    run::{Run, RunState},
};

const NOT_EXITED: i32 = 999;

/// An external executable that runs CI on request.
#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct Adapter {
    bin: PathBuf,
    env: HashMap<String, String>,
}

impl Adapter {
    pub fn new(bin: &Path) -> Self {
        Self {
            bin: bin.into(),
            env: HashMap::new(),
        }
    }

    pub fn with_environment(mut self, env: &HashMap<String, String>) -> Self {
        for (key, value) in env.iter() {
            self.env.insert(key.into(), value.into());
        }
        self
    }

    fn envs(&self) -> impl Iterator<Item = (&OsStr, &OsStr)> {
        self.env.iter().map(|(k, v)| (k.as_ref(), v.as_ref()))
    }

    pub fn run(
        &self,
        trigger: &Request,
        run: &mut Run,
        db: &Db,
        run_notification: &NotificationSender,
    ) -> Result<(), AdapterError> {
        run.set_state(RunState::Triggered);
        db.update_run(run).map_err(AdapterError::UpdateRun)?;

        let x = self.run_helper(trigger, run, db, run_notification);

        run.set_state(RunState::Finished);
        db.update_run(run).map_err(AdapterError::UpdateRun)?;

        x
    }

    fn run_helper(
        &self,
        trigger: &Request,
        run: &mut Run,
        db: &Db,
        run_notification: &NotificationSender,
    ) -> Result<(), AdapterError> {
        assert!(matches!(trigger, Request::Trigger { .. }));

        // Spawn the adapter sub-process.
        let mut child = Command::new(&self.bin)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .envs(self.envs())
            .spawn()
            .map_err(|e| AdapterError::SpawnAdapter(self.bin.clone(), e))?;

        run_notification.notify()?;

        // Write the request to trigger a run to the child's stdin.
        // Then close the pipe to prevent the child from trying to
        // read another message that will never be sent.
        {
            let stdin = child.stdin.take().ok_or(AdapterError::StdinHandle)?;
            trigger
                .to_writer(stdin)
                .map_err(AdapterError::RequestWrite)?;
        }

        // Get the child's stdout into a BufReader so that we can loop
        // over lines.
        let stdout = child.stdout.take().ok_or(AdapterError::StdoutHandle)?;
        let stdout = BufReader::new(stdout);
        let mut lines = stdout.lines();

        if let Some(line) = lines.next() {
            let line = line.map_err(AdapterError::ReadLine)?;
            let resp = Response::from_str(&line).map_err(AdapterError::ParseResponse)?;
            run_notification.notify()?;
            match resp {
                Response::Triggered { run_id, info_url } => {
                    run.set_state(RunState::Running);
                    run.set_adapter_run_id(run_id);
                    if let Some(url) = info_url {
                        run.set_adapter_info_url(&url);
                    }
                    db.update_run(run).map_err(AdapterError::UpdateRun)?;
                }
                _ => return Err(AdapterError::NotTriggered(resp)),
            }
        } else {
            logger::adapter_no_first_response();
        }

        if let Some(line) = lines.next() {
            let line = line.map_err(AdapterError::ReadLine)?;
            let resp = Response::from_str(&line).map_err(AdapterError::ParseResponse)?;
            run_notification.notify()?;
            match resp {
                Response::Finished { result } => {
                    run.set_result(result);
                    db.update_run(run).map_err(AdapterError::UpdateRun)?;
                }
                _ => return Err(AdapterError::NotFinished(resp)),
            }
        } else {
            logger::adapter_no_second_response();
        }

        if let Some(line) = lines.next() {
            let line = line.map_err(AdapterError::ReadLine)?;
            let resp = Response::from_str(&line).map_err(AdapterError::ParseResponse)?;
            logger::adapter_too_many_responses();
            return Err(AdapterError::TooMany(resp));
        }

        let wait = child.wait().map_err(AdapterError::Wait)?;

        let mut stderr = child.stderr.take().ok_or(AdapterError::StderrHandle)?;
        let mut buf = vec![];
        stderr
            .read_to_end(&mut buf)
            .map_err(AdapterError::ReadStderr)?;
        let stderr = String::from_utf8_lossy(&buf);
        logger::adapter_result(wait.code(), &stderr);

        if let Some(exit) = wait.code() {
            if exit != 0 {
                return Err(AdapterError::Failed(exit));
            }
        } else {
            logger::adapter_did_not_exit_voluntarily();
            return Err(AdapterError::Failed(NOT_EXITED));
        }

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    /// Error creating Response from a string.
    #[error("failed to create a Response message from adapter output")]
    ParseResponse(#[source] MessageError),

    /// Request writing failure.
    #[error("failed to write request to adapter stdin")]
    RequestWrite(#[source] MessageError),

    /// Error from spawning a sub-process.
    #[error("failed to spawn a CI adapter sub-process: {0}")]
    SpawnAdapter(PathBuf, #[source] std::io::Error),

    /// Error getting the file handle for the adapter's stdin.
    #[error("failed to get handle for adapter's stdin")]
    StdinHandle,

    /// Error getting the file handle for the adapter's stdout.
    #[error("failed to get handle for adapter's stdout")]
    StdoutHandle,

    /// Error getting the file handle for the adapter's stderr.
    #[error("failed to get handle for adapter's stderr")]
    StderrHandle,

    /// Error reading adapter's stderr.
    #[error("failed to read the adapter's stderr")]
    ReadStderr(#[source] std::io::Error),

    #[error("failed to read from adapter stdout")]
    ReadLine(#[source] std::io::Error),

    /// Waiting for child process failed.
    #[error("failed to wait for child process to exit")]
    Wait(#[source] std::io::Error),

    /// Child process failed.
    #[error("child process failed with wait status {0}")]
    Failed(i32),

    /// First message is not `Response::Triggered`
    #[error("adapter's first message is not 'triggered', but {0:?}")]
    NotTriggered(Response),

    /// Second message is not `Response::Finished`
    #[error("adapter's second message is not 'finished', but {0:?}")]
    NotFinished(Response),

    /// Too many messages from adapter.
    #[error("adapter sent too many messages: first extra is {0:#?}")]
    TooMany(Response),

    /// Could no update run in database.
    #[error("failed to update CI run information in database")]
    UpdateRun(#[source] DbError),

    /// Could not send notification of changes to CI runs.
    #[error(transparent)]
    Notif(#[from] crate::notif::NotificationError),
}

#[cfg(test)]
mod test {
    use std::{fs::write, io::ErrorKind};

    use tempfile::{tempdir, NamedTempFile};

    use radicle::git::Oid;
    use radicle::prelude::RepoId;

    use super::{Adapter, Db, Run};
    use crate::{
        adapter::AdapterError,
        msg::{MessageError, Response, RunResult},
        notif::NotificationChannel,
        run::Whence,
        test::{mock_adapter, trigger_request, TestResult},
    };

    fn db() -> anyhow::Result<Db> {
        let tmp = NamedTempFile::new()?;
        let db = Db::new(tmp.path())?;
        Ok(db)
    }

    fn run() -> anyhow::Result<Run> {
        Ok(Run::new(
            RepoId::from_urn("rad:zwTxygwuz5LDGBq255RA2CbNGrz8")?,
            "test.repo",
            Whence::branch(
                "main",
                Oid::try_from("ff3099ba5de28d954c41d0b5a84316f943794ea4")?,
                Some("J. Random Hacker <random@example.com>"),
            ),
            "2024-02-29T12:58:12+02:00".into(),
        ))
    }

    #[test]
    fn adapter_reports_success() -> TestResult<()> {
        const ADAPTER: &str = r#"#!/bin/bash
read
echo '{"response":"triggered","run_id":{"id":"xyzzy"}}'
echo '{"response":"finished","result":"success"}'
"#;

        let tmp = tempdir()?;
        let bin = tmp.path().join("adapter.sh");
        mock_adapter(&bin, ADAPTER)?;

        let db = db()?;
        let mut run = run()?;
        let mut channel = NotificationChannel::new_run();
        let sender = channel.tx()?;
        Adapter::new(&bin).run(&trigger_request()?, &mut run, &db, &sender)?;
        assert_eq!(run.result(), Some(&RunResult::Success));

        Ok(())
    }

    #[test]
    fn adapter_reports_failure() -> TestResult<()> {
        const ADAPTER: &str = r#"#!/bin/bash
read
echo '{"response":"triggered","run_id":{"id":"xyzzy"}}'
echo '{"response":"finished","result":"failure"}'
"#;

        let tmp = tempdir()?;
        let bin = tmp.path().join("adapter.sh");
        mock_adapter(&bin, ADAPTER)?;

        let db = db()?;
        let mut run = run()?;
        let mut channel = NotificationChannel::new_run();
        let sender = channel.tx()?;
        let x = Adapter::new(&bin).run(&trigger_request()?, &mut run, &db, &sender);

        match x {
            Ok(_) => (),
            Err(AdapterError::RequestWrite(_)) => (),
            _ => panic!("unexpected result: {x:#?}"),
        }

        Ok(())
    }

    #[test]
    fn adapter_exits_nonzero() -> TestResult<()> {
        const ADAPTER: &str = r#"#!/bin/bash
read
echo '{"response":"triggered","run_id":{"id":"xyzzy"}}'
echo '{"response":"finished","result":"failure"}'
echo woe be me 1>&2
exit 1
"#;

        let tmp = tempdir()?;
        let bin = tmp.path().join("adapter.sh");
        mock_adapter(&bin, ADAPTER)?;

        let db = db()?;
        let mut run = run()?;
        let mut channel = NotificationChannel::new_run();
        let sender = channel.tx()?;
        let x = Adapter::new(&bin).run(&trigger_request()?, &mut run, &db, &sender);
        eprintln!("{x:#?}");
        assert!(x.is_err());
        assert_eq!(run.result(), Some(&RunResult::Failure));

        Ok(())
    }

    #[test]
    fn adapter_is_killed_before_any_messages() -> TestResult<()> {
        const ADAPTER: &str = r#"#!/bin/bash
kill -9 $BASHPID
"#;

        let tmp = tempdir()?;
        let bin = tmp.path().join("adapter.sh");
        mock_adapter(&bin, ADAPTER)?;

        let db = db()?;
        let mut run = run()?;
        let mut channel = NotificationChannel::new_run();
        let sender = channel.tx()?;
        let x = Adapter::new(&bin).run(&trigger_request()?, &mut run, &db, &sender);
        eprintln!("{x:#?}");
        assert!(matches!(
            x,
            Err(AdapterError::Failed(_)) | Err(AdapterError::RequestWrite(_))
        ));

        Ok(())
    }

    #[test]
    fn adapter_is_killed_after_first_message() -> TestResult<()> {
        const ADAPTER: &str = r#"#!/bin/bash
read
echo '{"response":"triggered","run_id":{"id":"xyzzy"}}'
kill -9 $BASHPID
"#;

        let tmp = tempdir()?;
        let bin = tmp.path().join("adapter.sh");
        mock_adapter(&bin, ADAPTER)?;

        let db = db()?;
        let mut run = run()?;
        let mut channel = NotificationChannel::new_run();
        let sender = channel.tx()?;
        let x = Adapter::new(&bin).run(&trigger_request()?, &mut run, &db, &sender);
        eprintln!("{x:#?}");
        assert!(matches!(x, Err(AdapterError::Failed(_))));

        Ok(())
    }

    #[test]
    fn adapter_is_killed_after_second_message() -> TestResult<()> {
        const ADAPTER: &str = r#"#!/bin/bash
read
echo '{"response":"triggered","run_id":{"id":"xyzzy"}}'
echo '{"response":"finished","result":"success"}'
kill -9 $BASHPID
"#;

        let tmp = tempdir()?;
        let bin = tmp.path().join("adapter.sh");
        mock_adapter(&bin, ADAPTER)?;

        let db = db()?;
        let mut run = run()?;
        let mut channel = NotificationChannel::new_run();
        let sender = channel.tx()?;
        let x = Adapter::new(&bin).run(&trigger_request()?, &mut run, &db, &sender);
        eprintln!("{x:#?}");
        assert!(matches!(x, Err(AdapterError::Failed(_))));

        Ok(())
    }

    #[test]
    fn adapter_produces_as_bad_message() -> TestResult<()> {
        const ADAPTER: &str = r#"#!/bin/bash
read
echo '{"response":"triggered","run_id":{"id":"xyzzy"}}'
echo '{"response":"finished","result":"success","bad":"field"}'
"#;

        let tmp = tempdir()?;
        let bin = tmp.path().join("adapter.sh");
        mock_adapter(&bin, ADAPTER)?;

        let db = db()?;
        let mut run = run()?;
        let mut channel = NotificationChannel::new_run();
        let sender = channel.tx()?;
        let x = Adapter::new(&bin).run(&trigger_request()?, &mut run, &db, &sender);

        match x {
            Err(AdapterError::ParseResponse(MessageError::DeserializeResponse(_))) => (),
            _ => panic!("unexpected result: {x:#?}"),
        }

        Ok(())
    }

    #[test]
    fn adapter_first_message_isnt_triggered() -> TestResult<()> {
        const ADAPTER: &str = r#"#!/bin/bash
read
echo '{"response":"finished","result":"success"}'
"#;

        let tmp = tempdir()?;
        let bin = tmp.path().join("adapter.sh");
        mock_adapter(&bin, ADAPTER)?;

        let db = db()?;
        let mut run = run()?;
        let mut channel = NotificationChannel::new_run();
        let sender = channel.tx()?;
        let x = Adapter::new(&bin).run(&trigger_request()?, &mut run, &db, &sender);
        eprintln!("{x:#?}");
        assert!(matches!(
            x,
            Err(AdapterError::NotTriggered(Response::Finished {
                result: RunResult::Success
            }))
        ));

        Ok(())
    }

    #[test]
    fn adapter_outputs_too_many_messages() -> TestResult<()> {
        const ADAPTER: &str = r#"#!/bin/bash
read
echo '{"response":"triggered","run_id":{"id":"xyzzy"}}'
echo '{"response":"finished","result":"success"}'
echo '{"response":"finished","result":"success"}'
"#;

        let tmp = tempdir()?;
        let bin = tmp.path().join("adapter.sh");
        mock_adapter(&bin, ADAPTER)?;

        let db = db()?;
        let mut run = run()?;
        let mut channel = NotificationChannel::new_run();
        let sender = channel.tx()?;
        let x = Adapter::new(&bin).run(&trigger_request()?, &mut run, &db, &sender);
        eprintln!("{x:#?}");
        assert!(matches!(
            x,
            Err(AdapterError::TooMany(Response::Finished {
                result: RunResult::Success
            }))
        ));

        Ok(())
    }

    #[test]
    fn adapter_does_not_exist() -> TestResult<()> {
        let tmp = tempdir()?;
        let bin = tmp.path().join("adapter.sh");

        let db = db()?;
        let mut run = run()?;
        let mut channel = NotificationChannel::new_run();
        let sender = channel.tx()?;
        let x = Adapter::new(&bin).run(&trigger_request()?, &mut run, &db, &sender);
        eprintln!("{x:#?}");
        match x {
            Err(AdapterError::SpawnAdapter(filename, e)) => {
                assert_eq!(bin, filename);
                assert_eq!(e.kind(), ErrorKind::NotFound);
            }
            _ => panic!("expected a specific error"),
        }

        Ok(())
    }

    #[test]
    fn adapter_is_not_executable() -> TestResult<()> {
        const ADAPTER: &str = r#"#!/bin/bash
read
echo '{"response":"triggered","run_id":{"id":"xyzzy"}}'
echo '{"response":"finished","result":"success"}'
"#;

        let tmp = tempdir()?;
        let bin = tmp.path().join("adapter.sh");
        write(&bin, ADAPTER)?;

        let db = db()?;
        let mut run = run()?;
        let mut channel = NotificationChannel::new_run();
        let sender = channel.tx()?;
        let x = Adapter::new(&bin).run(&trigger_request()?, &mut run, &db, &sender);
        eprintln!("{x:#?}");
        match x {
            Err(AdapterError::SpawnAdapter(filename, e)) => {
                assert_eq!(bin, filename);
                assert_eq!(e.kind(), ErrorKind::PermissionDenied);
            }
            _ => panic!("expected a specific error"),
        }

        Ok(())
    }

    #[test]
    fn adapter_has_bad_interpreter() -> TestResult<()> {
        // We test this with a shebang. However, the same kind of code
        // paths and errors should happen when a binary can't be
        // loaded due to missing dynamic linker or library or such.

        const ADAPTER: &str = r#"#!/bin/does-not-exist
read
echo '{"response":"triggered","run_id":{"id":"xyzzy"}}'
echo '{"response":"finished","result":"success"}'
"#;

        let tmp = tempdir()?;
        let bin = tmp.path().join("adapter.sh");
        mock_adapter(&bin, ADAPTER)?;

        let db = db()?;
        let mut run = run()?;
        let mut channel = NotificationChannel::new_run();
        let sender = channel.tx()?;
        let x = Adapter::new(&bin).run(&trigger_request()?, &mut run, &db, &sender);
        eprintln!("{x:#?}");
        match x {
            Err(AdapterError::SpawnAdapter(filename, e)) => {
                assert_eq!(bin, filename);
                assert_eq!(e.kind(), ErrorKind::NotFound);
            }
            _ => panic!("expected a specific error"),
        }

        Ok(())
    }
}
