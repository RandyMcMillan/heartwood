//! Database abstraction for the Radicle CI broker.
//!
//! This module is a wrapper around the [SQLite](https://sqlite.org/)
//! database. It is meant to suffice for the Radicle CI broker, and
//! does not try to be a more generic wrapper.
//!
//! The database stores the following kinds of data:
//!
//! - "counter": This is used for testing the implementation of
//!   concurrent access to the database. It is not useful for anything
//!   else.

use std::{
    fmt,
    path::{Path, PathBuf},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use sqlite::{Connection, State, Statement};
use time::{macros::format_description, OffsetDateTime};
use uuid::Uuid;

use crate::{ci_event::CiEvent, msg::RunId, run::Run};

// how long to retry when SQL fails for busy database
const MAX_WAIT: Duration = Duration::from_millis(60_000);

/// The CI broker database. It stores the data that needs to be
/// persistent, even if the process terminates.
pub struct Db {
    filename: PathBuf,
    conn: Connection,
}

impl Db {
    /// Open or create a database. It is created if it doesn't exist.
    /// If it is created, tables are created.
    pub fn new<P: AsRef<Path>>(filename: P) -> Result<Self, DbError> {
        let filename = filename.as_ref();
        let mut db = Db {
            filename: filename.into(),
            conn: sqlite::open(filename).map_err(|e| DbError::open(filename, e))?,
        };

        debug_assert!(MAX_WAIT.as_millis() < usize::MAX as u128); // safe conversion
        let ms = MAX_WAIT.as_millis() as usize;
        db.conn
            .set_busy_timeout(ms)
            .map_err(|e| DbError::busy_timer(filename, e))?;

        db.create_tables()?;

        Ok(db)
    }

    fn create_tables(&self) -> Result<(), DbError> {
        const TABLES: &[&str] = &[
            "CREATE TABLE IF NOT EXISTS counter_test (counter INT)",
            "CREATE TABLE IF NOT EXISTS event_queue (id TEXT PRIMARY KEY, timestamp TEXT, event TEXT)",
            "CREATE TABLE IF NOT EXISTS ci_event_queue (id TEXT PRIMARY KEY, timestamp TEXT, event TEXT)",
            "CREATE TABLE IF NOT EXISTS ci_runs (broker_run_id TEXT PRIMARY KEY, json TEXT)",
        ];

        for table in TABLES.iter() {
            let mut stmt = self.prepare(table)?;
            Self::execute_valueless(&mut stmt)?;
        }

        Ok(())
    }

    /// Name of database file.
    pub fn filename(&self) -> &Path {
        &self.filename
    }

    /// Start a transaction.
    pub fn begin(&self) -> Result<(), DbError> {
        let mut stmt = self.prepare("BEGIN TRANSACTION")?;
        Self::execute_valueless(&mut stmt)
    }

    /// Commit a transaction.
    pub fn commit(&self) -> Result<(), DbError> {
        let mut stmt = self.prepare("COMMIT")?;
        Self::execute_valueless(&mut stmt)
    }

    /// Roll back a transaction.
    pub fn rollback(&self) -> Result<(), DbError> {
        let mut stmt = self.prepare("ROLLBACK")?;
        Self::execute_valueless(&mut stmt)
    }

    // Prepare a statement for execution.
    fn prepare<'a>(&'a self, sql: &str) -> Result<Stmt<'a>, DbError> {
        match self.conn.prepare(sql) {
            Ok(stmt) => Ok(Stmt::new(sql, stmt)),
            Err(e) => Err(DbError::prepare(sql, &self.filename, e)),
        }
    }

    // Execute a statement that doesn't return any rows with values.
    // This means basically any statement except SELECT.
    fn execute_valueless(stmt: &mut Stmt) -> Result<(), DbError> {
        stmt.stmt.reset().map_err(DbError::reset)?;
        match stmt.stmt.next() {
            Ok(_) => Ok(()),
            Err(e) => Err(DbError::execute(&stmt.sql, e)),
        }
    }

    /// Create the counter with an initial value. Only use this if
    /// there isn't a counter row already.
    pub fn create_counter(&self, counter: i64) -> Result<(), DbError> {
        let sql = "INSERT INTO counter_test (counter) VALUES (:1)";
        let mut insert = self.prepare(sql)?;
        insert
            .stmt
            .bind((1, counter))
            .map_err(|e| DbError::bind(sql, e))?;
        match insert.stmt.next() {
            Ok(_) => (),
            Err(e) => return Err(DbError::insert_counter(&insert.sql, e)),
        }
        Ok(())
    }

    /// Update the counter to have a new value.
    pub fn update_counter(&self, counter: i64) -> Result<(), DbError> {
        let sql = "UPDATE counter_test SET counter = :1";
        let mut update = self.prepare(sql)?;
        update
            .stmt
            .bind((1, counter))
            .map_err(|e| DbError::bind(sql, e))?;
        match update.stmt.next() {
            Ok(_) => (),
            Err(e) => return Err(DbError::update_counter(&update.sql, e)),
        }
        Ok(())
    }

    /// Return the current value of the counter, if any.
    pub fn get_counter(&self) -> Result<Option<i64>, DbError> {
        let sql = "SELECT counter FROM counter_test";
        let mut select = self.prepare(sql)?;
        let mut counter = None;

        loop {
            match select.stmt.next() {
                Ok(State::Row) => {
                    counter = Some(
                        select
                            .stmt
                            .read("counter")
                            .map_err(|e| DbError::read(sql, e))?,
                    );
                }
                Ok(State::Done) => {
                    break;
                }
                Err(e) => {
                    return Err(DbError::get_counter(&select.sql, e));
                }
            }
        }

        Ok(counter)
    }

    /// Return list of identifiers for broker events currently in the queue.
    ///
    /// Note that there is no longer any way to retrieve the broker
    /// events. This method is meant only for making sure there are no
    /// unprocessed broker events in the queue.
    pub fn queued_events(&self) -> Result<Vec<QueueId>, DbError> {
        let mut select = self.prepare("SELECT id FROM event_queue")?;

        let mut ids = vec![];

        loop {
            match select.stmt.next() {
                Ok(State::Row) => {
                    let id: String = select
                        .stmt
                        .read("id")
                        .map_err(|e| DbError::list_events(&select.sql, e))?;
                    ids.push(QueueId::from(&id));
                }
                Ok(State::Done) => {
                    break;
                }
                Err(e) => {
                    return Err(DbError::list_events(&select.sql, e));
                }
            }
        }

        Ok(ids)
    }

    /// Return list of identifiers for CI events currently in the queue.
    pub fn queued_ci_events(&self) -> Result<Vec<QueueId>, DbError> {
        let mut select = self.prepare("SELECT id FROM ci_event_queue")?;

        let mut ids = vec![];

        loop {
            match select.stmt.next() {
                Ok(State::Row) => {
                    let id: String = select
                        .stmt
                        .read("id")
                        .map_err(|e| DbError::list_events(&select.sql, e))?;
                    ids.push(QueueId::from(&id));
                }
                Ok(State::Done) => {
                    break;
                }
                Err(e) => {
                    return Err(DbError::list_events(&select.sql, e));
                }
            }
        }

        Ok(ids)
    }

    /// Return a specific event, given is id, if one exists.
    pub fn get_queued_ci_event(&self, id: &QueueId) -> Result<Option<QueuedCiEvent>, DbError> {
        let mut select =
            self.prepare("SELECT timestamp, event FROM ci_event_queue WHERE id = :id")?;
        select
            .stmt
            .bind((":id", id.as_str()))
            .map_err(|e| DbError::bind(&select.sql, e))?;

        let mut timestamp: Option<String> = None;
        let mut event: Option<CiEvent> = None;

        loop {
            match select.stmt.next() {
                Ok(State::Row) => {
                    timestamp = Some(
                        select
                            .stmt
                            .read("timestamp")
                            .map_err(|e| DbError::get_event(&select.sql, e))?,
                    );
                    let json: String = select
                        .stmt
                        .read("event")
                        .map_err(|e| DbError::get_event(&select.sql, e))?;
                    event = Some(
                        serde_json::from_str(&json)
                            .map_err(|e| DbError::event_from_json(&json, e))?,
                    );
                }
                Ok(State::Done) => {
                    break;
                }
                Err(e) => {
                    return Err(DbError::get_event(&select.sql, e));
                }
            }
        }

        if let (Some(ts), Some(ev)) = (timestamp, event) {
            let qe = QueuedCiEvent::new(id.clone(), ts, ev);
            Ok(Some(qe))
        } else {
            Ok(None)
        }
    }

    /// Add a new event to the event queue, returning its id.
    pub fn push_queued_ci_event(&self, event: CiEvent) -> Result<QueueId, DbError> {
        let json = serde_json::to_string(&event).map_err(DbError::event_to_json)?;

        let id = QueueId::default();
        let ts = now().map_err(DbError::time_format)?;

        let mut insert = self
            .prepare("INSERT INTO ci_event_queue (id, timestamp, event) VALUES (:id, :ts, :e)")?;
        insert
            .stmt
            .bind((":id", id.as_str()))
            .map_err(|e| DbError::bind(&insert.sql, e))?;
        insert
            .stmt
            .bind((":ts", ts.as_str()))
            .map_err(|e| DbError::bind(&insert.sql, e))?;
        insert
            .stmt
            .bind((":e", json.as_str()))
            .map_err(|e| DbError::bind(&insert.sql, e))?;
        match insert.stmt.next() {
            Ok(_) => (),
            Err(e) => return Err(DbError::push_event(&insert.sql, e)),
        }

        Ok(id)
    }

    /// Remove CI event from queue, given its id. It's OK if the event is
    /// not in the queue, that is just silently ignored.
    pub fn remove_queued_ci_event(&self, id: &QueueId) -> Result<(), DbError> {
        let mut remove = self.prepare("DELETE FROM ci_event_queue WHERE id = :id")?;
        remove
            .stmt
            .bind((":id", id.as_str()))
            .map_err(|e| DbError::bind(&remove.sql, e))?;

        match remove.stmt.next() {
            Ok(_) => (),
            Err(e) => return Err(DbError::remove_event(&remove.sql, e)),
        }

        Ok(())
    }

    /// Return list of CI runs currently in the database.
    pub fn list_runs(&self) -> Result<Vec<RunId>, DbError> {
        let mut select = self.prepare("SELECT broker_run_id FROM ci_runs")?;

        let mut run_ids = vec![];

        loop {
            let next = select.stmt.next();
            match next {
                Ok(State::Row) => {
                    let run_id: String = select
                        .stmt
                        .read("broker_run_id")
                        .map_err(|e| DbError::get_run(&select.sql, e))?;
                    let run_id = RunId::from(run_id.as_str());
                    run_ids.push(run_id);
                }
                Ok(State::Done) => {
                    break;
                }
                Err(e) => {
                    return Err(DbError::list_runs(&select.sql, e));
                }
            }
        }

        Ok(run_ids)
    }

    /// Return all CI runs currently in the database.
    pub fn get_all_runs(&self) -> Result<Vec<Run>, DbError> {
        let mut select = self.prepare("SELECT json FROM ci_runs")?;

        let mut runs = vec![];

        loop {
            let next = select.stmt.next();
            match next {
                Ok(State::Row) => {
                    let json: String = select
                        .stmt
                        .read("json")
                        .map_err(|e| DbError::get_run(&select.sql, e))?;
                    let run: Run = serde_json::from_str(&json)
                        .map_err(|e| DbError::run_from_json(&json, e))?;
                    runs.push(run);
                }
                Ok(State::Done) => {
                    break;
                }
                Err(e) => {
                    return Err(DbError::get_all_runs(&select.sql, e));
                }
            }
        }

        runs.sort_by_cached_key(|run| run.timestamp().to_string());
        Ok(runs)
    }

    /// Return a specific CI run, given is id, if one exists.
    pub fn get_run(&self, id: &RunId) -> Result<Option<Run>, DbError> {
        let mut select = self.prepare("SELECT json FROM ci_runs WHERE broker_run_id = :id")?;
        select
            .stmt
            .bind((":id", id.to_string().as_str()))
            .map_err(|e| DbError::bind(&select.sql, e))?;

        let mut run = None;

        select.stmt.reset().map_err(DbError::reset)?;
        loop {
            match select.stmt.next() {
                Ok(State::Row) => {
                    let json: String = select
                        .stmt
                        .read("json")
                        .map_err(|e| DbError::get_run(&select.sql, e))?;
                    run = Some(
                        serde_json::from_str(&json)
                            .map_err(|e| DbError::run_from_json(&json, e))?,
                    );
                }
                Ok(State::Done) => {
                    break;
                }
                Err(e) => {
                    return Err(DbError::get_run(&select.sql, e));
                }
            }
        }

        Ok(run)
    }

    /// Return a list of broker run IDs that their adapter run ID set
    /// to a given value.
    pub fn find_runs(&self, adapter_runid: &RunId) -> Result<Vec<Run>, DbError> {
        let runs = self
            .get_all_runs()?
            .iter()
            .filter(|run| run.adapter_run_id() == Some(adapter_runid))
            .cloned()
            .collect();

        Ok(runs)
    }

    /// Add a new CI run to the database, returning its id.
    pub fn push_run(&self, run: &Run) -> Result<RunId, DbError> {
        let id = run.broker_run_id().clone();

        let json = serde_json::to_string(&run).map_err(DbError::event_to_json)?;

        let mut insert =
            self.prepare("INSERT INTO ci_runs (broker_run_id, json) VALUES (:id, :json)")?;
        insert
            .stmt
            .bind((":id", id.to_string().as_str()))
            .map_err(|e| DbError::bind(&insert.sql, e))?;
        insert
            .stmt
            .bind((":json", json.as_str()))
            .map_err(|e| DbError::bind(&insert.sql, e))?;

        match insert.stmt.next() {
            Ok(_) => (),
            Err(e) => return Err(DbError::push_run(&insert.sql, e)),
        }

        Ok(id)
    }

    /// Update a CI run in the database.
    pub fn update_run(&self, run: &Run) -> Result<(), DbError> {
        let id = run.broker_run_id().clone();

        let json = serde_json::to_string(&run).map_err(DbError::event_to_json)?;

        let mut update =
            self.prepare("UPDATE ci_runs SET json = :json WHERE broker_run_id = :id")?;
        update
            .stmt
            .bind((":id", id.to_string().as_str()))
            .map_err(|e| DbError::bind(&update.sql, e))?;
        update
            .stmt
            .bind((":json", json.as_str()))
            .map_err(|e| DbError::bind(&update.sql, e))?;

        match update.stmt.next() {
            Ok(_) => (),
            Err(e) => return Err(DbError::update_run(&update.sql, e)),
        }

        Ok(())
    }

    /// Remove a CI run from database, given its id. It's OK if the run is
    /// not in the database, that is just silently ignored.
    pub fn remove_run(&self, id: &RunId) -> Result<(), DbError> {
        let mut remove = self.prepare("DELETE FROM ci_runs WHERE id = :id")?;
        remove
            .stmt
            .bind((":id", id.to_string().as_str()))
            .map_err(|e| DbError::bind(&remove.sql, e))?;

        match remove.stmt.next() {
            Ok(_) => (),
            Err(e) => return Err(DbError::remove_run(&remove.sql, e)),
        }

        Ok(())
    }
}

fn now() -> Result<String, time::error::Format> {
    let fmt =
        format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:6]Z");
    OffsetDateTime::now_utc().format(fmt)
}

// A wrapper around a statement that remembers its text form.
struct Stmt<'a> {
    sql: String,
    stmt: Statement<'a>,
}

impl<'a> Stmt<'a> {
    fn new(sql: &str, stmt: Statement<'a>) -> Self {
        Self {
            sql: sql.into(),
            stmt,
        }
    }
}

/// An identifier for an event in the event queue in the database.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueueId {
    id: String,
}

impl fmt::Display for QueueId {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.id)
    }
}

impl Default for QueueId {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
        }
    }
}

impl From<&str> for QueueId {
    fn from(id: &str) -> Self {
        Self { id: id.into() }
    }
}

impl From<&String> for QueueId {
    fn from(id: &String) -> Self {
        Self { id: id.into() }
    }
}

impl QueueId {
    fn as_str(&self) -> &str {
        self.id.as_str()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueuedCiEvent {
    id: QueueId,
    ts: String,
    event: CiEvent,
}

impl QueuedCiEvent {
    fn new(id: QueueId, ts: String, event: CiEvent) -> Self {
        Self { id, ts, event }
    }

    pub fn id(&self) -> &QueueId {
        &self.id
    }

    pub fn timestamp(&self) -> &str {
        &self.ts
    }

    pub fn event(&self) -> &CiEvent {
        &self.event
    }
}

/// All errors from this module.
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    /// Error formatting a time as a string.
    #[error(transparent)]
    Timeformat(#[from] time::error::Format),

    #[error("failed to set a busy timer one SQLite database {0}")]
    BusyTimer(PathBuf, #[source] sqlite::Error),

    #[error("failed to open SQLite database {0}")]
    Open(PathBuf, #[source] sqlite::Error),

    #[error("failed to prepare SQL statement SQLite database {0}: {1}")]
    Prepare(String, PathBuf, #[source] sqlite::Error),

    #[error("failed to reset connection to SQLite")]
    Reset(#[source] sqlite::Error),

    #[error("failed to execute SQL statement in SQLite: {0}")]
    Execute(String, #[source] sqlite::Error),

    #[error("failed to bind a value in SQL statement in SQLite: {0}")]
    Bind(String, #[source] sqlite::Error),

    #[error("failed to read a column value output of SQL statement in SQLite: {0}")]
    Read(String, #[source] sqlite::Error),

    #[error("failed to insert a counter into database")]
    InsertCounter(String, #[source] sqlite::Error),

    #[error("failed to update a counter in database")]
    UpdateCounter(String, #[source] sqlite::Error),

    #[error("failed to retrieve a counter from database")]
    GetCounter(String, #[source] sqlite::Error),

    #[error("failed to insert an event into database")]
    InsertEvent(String, #[source] sqlite::Error),

    #[error("failed to list queued events in database")]
    ListEvents(String, #[source] sqlite::Error),

    #[error("failed to retrieve a queued event in database")]
    GetEvent(String, #[source] sqlite::Error),

    #[error("failed to parse queued event as JSON: {0}")]
    EventFromJson(String, #[source] serde_json::Error),

    /// Can't convert broker event into JSON.
    #[error("failed to convert broker event into JSON")]
    EventToJson(#[source] serde_json::Error),

    #[error("failed to insert an event into queue")]
    PushEvent(String, #[source] sqlite::Error),

    #[error("failed to remove an event from queue")]
    RemoveEvent(String, #[source] sqlite::Error),

    #[error("failed to list CI runs in database")]
    ListRuns(String, #[source] sqlite::Error),

    #[error("failed to get all CI runs from database")]
    GetAllRuns(String, #[source] sqlite::Error),

    #[error("failed to parse CI run as JSON: {0}")]
    RunFromJson(String, #[source] serde_json::Error),

    #[error("failed to retrieve a CI run from database")]
    GetRun(String, #[source] sqlite::Error),

    #[error("failed to insert a CI run into database")]
    PushRun(String, #[source] sqlite::Error),

    #[error("failed to update a CI run in database")]
    UpdateRun(String, #[source] sqlite::Error),

    #[error("failed to remove a CI run from database")]
    RemoveRun(String, #[source] sqlite::Error),
}

impl DbError {
    fn time_format(e: time::error::Format) -> Self {
        Self::Timeformat(e)
    }

    fn busy_timer(filename: &Path, e: sqlite::Error) -> Self {
        Self::BusyTimer(filename.into(), e)
    }

    fn open(filename: &Path, e: sqlite::Error) -> Self {
        Self::Open(filename.into(), e)
    }

    fn prepare(sql: &str, filename: &Path, e: sqlite::Error) -> Self {
        Self::Prepare(sql.into(), filename.into(), e)
    }

    fn reset(e: sqlite::Error) -> Self {
        Self::Reset(e)
    }

    fn execute(sql: &str, e: sqlite::Error) -> Self {
        Self::Execute(sql.into(), e)
    }

    fn bind(sql: &str, e: sqlite::Error) -> Self {
        Self::Bind(sql.into(), e)
    }

    fn read(sql: &str, e: sqlite::Error) -> Self {
        Self::Read(sql.into(), e)
    }

    fn insert_counter(sql: &str, e: sqlite::Error) -> Self {
        Self::InsertCounter(sql.into(), e)
    }

    fn update_counter(sql: &str, e: sqlite::Error) -> Self {
        Self::UpdateCounter(sql.into(), e)
    }

    fn get_counter(sql: &str, e: sqlite::Error) -> Self {
        Self::GetCounter(sql.into(), e)
    }

    fn list_events(sql: &str, e: sqlite::Error) -> Self {
        Self::ListEvents(sql.into(), e)
    }

    fn get_event(sql: &str, e: sqlite::Error) -> Self {
        Self::GetEvent(sql.into(), e)
    }

    fn event_from_json(json: &str, e: serde_json::Error) -> Self {
        Self::EventFromJson(json.into(), e)
    }

    fn event_to_json(e: serde_json::Error) -> Self {
        Self::EventToJson(e)
    }

    fn push_event(sql: &str, e: sqlite::Error) -> Self {
        Self::PushEvent(sql.into(), e)
    }

    fn remove_event(sql: &str, e: sqlite::Error) -> Self {
        Self::RemoveEvent(sql.into(), e)
    }

    fn list_runs(sql: &str, e: sqlite::Error) -> Self {
        Self::ListRuns(sql.into(), e)
    }

    fn get_all_runs(sql: &str, e: sqlite::Error) -> Self {
        Self::GetAllRuns(sql.into(), e)
    }

    fn run_from_json(json: &str, e: serde_json::Error) -> Self {
        Self::RunFromJson(json.into(), e)
    }

    fn get_run(sql: &str, e: sqlite::Error) -> Self {
        Self::GetRun(sql.into(), e)
    }

    fn push_run(sql: &str, e: sqlite::Error) -> Self {
        Self::PushRun(sql.into(), e)
    }

    fn update_run(sql: &str, e: sqlite::Error) -> Self {
        Self::UpdateRun(sql.into(), e)
    }

    fn remove_run(sql: &str, e: sqlite::Error) -> Self {
        Self::RemoveRun(sql.into(), e)
    }
}
