//! A management tool for the CI broker.
//!
//! This tool lets the node operator query and manage the CI broker's
//! database: the persistent event queue, the list of CI runs, etc.
//!
//! The tool is also used as part of the CI broker's acceptance test
//! suite (see the `ci-broker.subplot` document).

#![allow(clippy::result_large_err)]
#![allow(unused_imports)] // FIXME

use std::{
    error::Error,
    fs::{read, write},
    path::PathBuf,
    process::exit,
    str::FromStr,
};

use clap::Parser;
use serde::Serialize;

use radicle::{
    git::RefString,
    prelude::{NodeId, RepoId},
    storage::ReadStorage,
    Profile, Storage,
};
use radicle_git_ext::Oid;

use radicle_ci_broker::{
    broker::BrokerError,
    ci_event::{CiEvent, CiEventError, CiEventV1},
    db::{Db, DbError, QueueId, QueuedCiEvent},
    logger,
    msg::{RunId, RunResult},
    notif::{NotificationChannel, NotificationError},
    pages::PageError,
    run::{Run, RunState, Whence},
    util::{self, UtilError},
};

mod cibtoolcmd;

fn main() {
    let _logger = logger::open();
    if let Err(e) = fallible_main() {
        eprintln!("ERROR: {}", e);
        let mut e = e.source();
        while let Some(source) = e {
            eprintln!("caused by: {}", source);
            e = source.source();
        }
        exit(1);
    }
}

#[allow(clippy::result_large_err)]
fn fallible_main() -> Result<(), CibToolError> {
    let args = Args::parse();
    args.cmd.run(&args)?;
    Ok(())
}

// A trait for subcommands with subcommands. The implementation of
// this trait would pick the subcommand value and execute its `run`
// method: either via this same trait for the subcommand value, or via
// the [`Leaf`] trait.
//
// Ideally, documentation comments meant to be used as help text by
// `clap` would be added to the enum types that list the subcommands,
// and types that implement `Leaf` trait, but that's only for
// consistency.
trait Subcommand {
    fn run(&self, args: &Args) -> Result<(), CibToolError>;
}

// A trait for subcommands with no subcommands. These do actual work,
// rather than providing a multi-level structure of subcommands. For
// `cibtool`, the leaf commands are in the `cibtoolcmd` module, and
// the higher level subcommands are in `cbitool.rs`.
trait Leaf {
    fn run(&self, args: &Args) -> Result<(), CibToolError>;
}

#[derive(Parser)]
#[command(version)]
struct Args {
    /// Name of the SQLite database file. The file will be created if
    /// it does not already exist. Locking is handled automatically.
    #[clap(long)]
    db: Option<PathBuf>,

    #[clap(subcommand)]
    cmd: Cmd,
}

impl Args {
    fn open_db(&self) -> Result<Db, CibToolError> {
        if let Some(filename) = &self.db {
            Ok(Db::new(filename)?)
        } else {
            Err(CibToolError::NoDb)
        }
    }
}

/// Radicle CI broker management tool for node operators.
///
/// Query and update the CI broker database file: the queue of events
/// waiting to be processed, the list of CI runs. Also, generate HTML
/// report pages from the database.
///
/// This tool can be used whether the CI broker is running or not.
#[derive(Parser)]
enum Cmd {
    #[clap(hide = true)]
    Counter(CounterCmd),
    Event(EventCmd),
    Run(RunCmd),
    Report(cibtoolcmd::ReportCmd),
    Trigger(cibtoolcmd::TriggerCmd),
}

impl Subcommand for Cmd {
    fn run(&self, args: &Args) -> Result<(), CibToolError> {
        match self {
            Self::Counter(x) => x.run(args),
            Self::Event(x) => x.run(args),
            Self::Run(x) => x.run(args),
            Self::Report(x) => x.run(args),
            Self::Trigger(x) => x.run(args),
        }
    }
}

#[derive(Parser)]
struct CounterCmd {
    #[clap(subcommand)]
    cmd: CounterSubCmd,
}

impl CounterCmd {
    fn run(&self, args: &Args) -> Result<(), CibToolError> {
        match &self.cmd {
            CounterSubCmd::Show(x) => x.run(args)?,
            CounterSubCmd::Count(x) => x.run(args)?,
        }
        Ok(())
    }
}

/// Manage a counter in the database. This is meant to be used
/// only by the CI broker test suite, not by people.
#[derive(Parser)]
enum CounterSubCmd {
    Show(cibtoolcmd::ShowCounter),
    Count(cibtoolcmd::CountCounter),
}

impl Subcommand for CounterSubCmd {
    fn run(&self, args: &Args) -> Result<(), CibToolError> {
        match self {
            Self::Show(x) => x.run(args),
            Self::Count(x) => x.run(args),
        }
    }
}

#[derive(Parser)]
struct EventCmd {
    #[clap(subcommand)]
    cmd: EventSubCmd,
}

impl Subcommand for EventCmd {
    fn run(&self, args: &Args) -> Result<(), CibToolError> {
        match &self.cmd {
            EventSubCmd::Add(x) => x.run(args)?,
            EventSubCmd::Shutdown(x) => x.run(args)?,
            EventSubCmd::List(x) => x.run(args)?,
            EventSubCmd::Count(x) => x.run(args)?,
            EventSubCmd::Show(x) => x.run(args)?,
            EventSubCmd::Remove(x) => x.run(args)?,
            EventSubCmd::Record(x) => x.run(args)?,
            EventSubCmd::Ci(x) => x.run(args)?,
            EventSubCmd::Filter(x) => x.run(args)?,
        }
        Ok(())
    }
}

/// Manage the event queue. The events are for Git refs having
/// changed.
#[derive(Parser)]
enum EventSubCmd {
    List(cibtoolcmd::ListEvents),
    Count(cibtoolcmd::CountEvents),
    Add(cibtoolcmd::AddEvent),
    Shutdown(cibtoolcmd::Shutdown),
    Show(cibtoolcmd::ShowEvent),
    Remove(cibtoolcmd::RemoveEvent),
    Record(cibtoolcmd::RecordEvents),
    Ci(cibtoolcmd::CiEvents),
    Filter(cibtoolcmd::FilterEvents),
}

#[derive(Parser)]
struct RunCmd {
    #[clap(subcommand)]
    cmd: RunSubCmd,
}

impl Subcommand for RunCmd {
    fn run(&self, args: &Args) -> Result<(), CibToolError> {
        match &self.cmd {
            RunSubCmd::Add(x) => x.run(args)?,
            RunSubCmd::List(x) => x.run(args)?,
            RunSubCmd::Update(x) => x.run(args)?,
            RunSubCmd::Show(x) => x.run(args)?,
        }
        Ok(())
    }
}

/// Manage the list of CI runs.
#[derive(Parser)]
enum RunSubCmd {
    /// Add information about a CI run to the database.
    Add(cibtoolcmd::AddRun),

    /// Update information about a CI run to the database.
    Update(cibtoolcmd::UpdateRun),

    /// Show a CI run as JSON.
    Show(cibtoolcmd::ShowRun),

    /// List known CI runs on this node to the database.
    List(cibtoolcmd::ListRuns),
}

#[derive(Debug, thiserror::Error)]
enum CibToolError {
    #[error("failed to look up node profile")]
    Profile(#[source] radicle::profile::Error),

    #[error("failed to look up open node storage")]
    Storage(#[source] radicle::storage::Error),

    #[error("failed to list repositories in node storage")]
    Repositories(#[source] radicle::storage::Error),

    #[error("failed to look up project info for repository {0}")]
    Project(RepoId, #[source] radicle::identity::doc::PayloadError),

    #[error("node has more than one repository called {0}")]
    DuplicateRepositories(String),

    #[error("node has no repository called: {0}")]
    NotFound(String),

    #[error("cannot find CI run with id {0}")]
    RunNotFound(RunId),

    #[error("failed to open git repository in node storage: {0}")]
    RepoOpen(RepoId, #[source] radicle::storage::RepositoryError),

    #[error("failed to parse git ref as a commit id: {0}")]
    RevParse(String, #[source] radicle::git::raw::Error),

    #[error(transparent)]
    Broker(#[from] BrokerError),

    #[error(transparent)]
    Db(#[from] DbError),

    #[error("failed to serialize CI event to JSON: {0:#?}")]
    EventToJson(CiEvent, #[source] serde_json::Error),

    #[error("failed to serialize node event to JSON: {0:#?}")]
    NodeEevnetToJson(radicle::node::Event, #[source] serde_json::Error),

    #[error("failed to serialize list of stored broker event to JSON")]
    EventListToJson(#[source] serde_json::Error),

    #[error("failed to serialize CI run information to JSON")]
    RunToJson(#[source] serde_json::Error),

    #[error("failed to write file: {0}")]
    Write(PathBuf, #[source] std::io::Error),

    #[error(transparent)]
    Page(#[from] PageError),

    #[error("failed to interpret as a Git reference: {0:?}")]
    RefString(String, #[source] radicle::git::fmt::Error),

    #[error("failed to read event ID from file {0}")]
    ReadEventId(PathBuf, #[source] std::io::Error),

    #[error("failed to write event ID to file {0}")]
    WriteEventId(PathBuf, #[source] std::io::Error),

    #[error("one of --id or --id-file or --all is required")]
    MissingId,

    #[error("programming error: confused about state and result of run")]
    AddRunConfusion,

    #[error(transparent)]
    Util(#[from] UtilError),

    #[error("no database file specified with the --db option")]
    NoDb,

    #[error("failed to subscribe to node events")]
    EventSubscribe(#[source] radicle_ci_broker::node_event_source::NodeEventError),

    #[error("failed to get next node event")]
    GetNodeEvent(#[source] radicle_ci_broker::node_event_source::NodeEventError),

    #[error("failed to create file for node events: {0}")]
    CreateEventsFile(PathBuf, #[source] std::io::Error),

    #[error("failed to write node event to file {0}")]
    WriteEvent(PathBuf, #[source] std::io::Error),

    #[error("failed to read node events from file {0}")]
    ReadEvents(PathBuf, #[source] std::io::Error),

    #[error("failed to read broker events from file {0}")]
    ReadBrokerEvents(PathBuf, #[source] std::io::Error),

    #[error("failed to read node events as UTF8 from file {0}")]
    NodeEventNotUtf8(PathBuf, #[source] std::string::FromUtf8Error),

    #[error("failed to read broker events as UTF8 from file {0}")]
    BrokerEventNotUtf8(PathBuf, #[source] std::string::FromUtf8Error),

    #[error("failed to read node events as JSON from file {0}")]
    JsonToNodeEvent(PathBuf, #[source] serde_json::Error),

    #[error("failed to create file for broker events: {0}")]
    CreateBrokerEventsFile(PathBuf, #[source] std::io::Error),

    #[error("failed to read filters from YAML file {0}")]
    ReadFilters(PathBuf, #[source] radicle_ci_broker::filter::FilterError),

    #[error("failed to construct a CiEvent::BranchCreated")]
    BranchCreted(#[source] CiEventError),

    #[error("programming error: failed to set up inter-thread notification channel")]
    Notification(#[source] NotificationError),
}
