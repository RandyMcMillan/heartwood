//! A logger abstraction for the CI broker.

#[cfg(test)]
use std::sync::Once;
use std::{
    fmt,
    io::Write,
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};

use clap::ValueEnum;
use radicle::{git::raw::Oid, identity::RepoId, node::Event};
use slog::{debug, error, info, o, trace, warn, Drain};
use slog_scope::GlobalLoggerGuard;

use crate::{
    ci_event::CiEvent,
    ci_event_source::CiEventSource,
    config::Config,
    db::{QueueId, QueuedCiEvent},
    msg::Request,
    node_event_source::NodeEventSource,
    run::Run,
};

// We define our own type for log levels so that we can apply
// clap::ValueEnum on it.
#[derive(ValueEnum, Eq, PartialEq, Copy, Clone, Debug)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{:?}>", self)
    }
}

impl From<LogLevel> for slog::Level {
    fn from(log_level: LogLevel) -> Self {
        match log_level {
            LogLevel::Trace => slog::Level::Trace,
            LogLevel::Debug => slog::Level::Debug,
            LogLevel::Info => slog::Level::Info,
            LogLevel::Warning => slog::Level::Warning,
            LogLevel::Error => slog::Level::Error,
            LogLevel::Critical => slog::Level::Critical,
        }
    }
}

// A custom type for log output. We need this to hide log messages
// from output in successful tests. Cargo will hide standard output
// and error by default, which is what we want, but it seems this only
// applies to output via the `print!` family of macros. If we open a
// new handle for stdout or stderr, output via that handle is not
// captured by Cargo.
//
// Instead, we have our own log writer type that writes to stderr in
// production mode, and uses `print!` in tests.
#[derive(Default)]
struct LogWriter {}

#[cfg(test)]
impl Write for LogWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let s = String::from_utf8_lossy(buf);
        print!("{s}");
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(not(test))]
impl Write for LogWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut stderr = std::io::stderr();
        stderr.write_all(buf)?;
        stderr.flush()?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub struct Logger {
    minimum_log_level: Arc<Mutex<slog::Level>>,

    // We need to keep this for as long as need to log. But we don't
    // refer to it directly.
    #[allow(dead_code)]
    guard: GlobalLoggerGuard,
}

impl Logger {
    pub fn set_minimum_level(&self, level: LogLevel) {
        *self
            .minimum_log_level
            .lock()
            .expect("set minimum log level") = level.into();
    }
}

pub fn open() -> Logger {
    let underlying_logger = slog_json::Json::new(LogWriter::default())
        .add_default_keys()
        .set_flush(true)
        .set_newlines(true)
        .build();

    // Set the default log level. Trace is good for tests.
    let level = Arc::new(Mutex::new(LogLevel::Trace.into()));

    let filter = LogLevelFilter::new(underlying_logger, level.clone());
    let log = slog::Logger::root(Mutex::new(filter).fuse(), o!());
    let guard = slog_scope::set_global_logger(log);

    Logger {
        guard,
        minimum_log_level: level,
    }
}

// Set up structured logging for tests.
//
// We have to keep the GlobalLoggerGuard we get when we initialize
// `slog-scope` alive as long as we may need to do any logging. We can
// only create that once per process. For tests, we do that here.
//
// We can't do the same thing for non-test processes, as that would
// interfere with use of `radicle-ci-broker` as a library. Libraries
// should not interfere with global state, unless they're specifically
// intended to do that.
//
// This is for tests only: otherwise the default global logger is
// used, and that's OK for a long-running process.

// We use this to make sure we initialize the logger only once.
#[cfg(test)]
static INIT: Once = Once::new();

// This is the actual logger for tests.
#[cfg(test)]
static mut LOGGER: Option<Logger> = None;

struct LogLevelFilter<D> {
    drain: D,
    minimum_log_level: Arc<Mutex<slog::Level>>,
}

impl<D> LogLevelFilter<D> {
    pub fn new(drain: D, minimum_log_level: Arc<Mutex<slog::Level>>) -> Self {
        Self {
            drain,
            minimum_log_level,
        }
    }
}

impl<D> Drain for LogLevelFilter<D>
where
    D: Drain,
{
    type Ok = Option<D::Ok>;
    type Err = Option<D::Err>;

    fn log(
        &self,
        record: &slog::Record,
        values: &slog::OwnedKVList,
    ) -> Result<Self::Ok, Self::Err> {
        #[allow(clippy::unwrap_used)] // We have no good way to report an error
        let min = *self
            .minimum_log_level
            .lock()
            .expect("lock log level filter minimum level");
        if record.level().is_at_least(min) {
            self.drain.log(record, values).map(Some).map_err(Some)
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
#[ctor::ctor]
fn open_for_tests() {
    INIT.call_once(|| unsafe {
        LOGGER = Some(open());
    });
}

pub fn start_cib() {
    info!(slog_scope::logger(), "CI broker starts"; "version" => env!("GIT_HEAD"));
}

pub fn end_cib_successfully() {
    info!(slog_scope::logger(), "CI broker ends successfully");
}

pub fn end_cib_in_error() {
    error!(
        slog_scope::logger(),
        "CI broker ends in unrecoverable error"
    );
}

pub fn node_event_source_created(source: &NodeEventSource) {
    info!(
        slog_scope::logger(),
        "created node event source";
        "source" => format!("{source:#?}")
    );
}

pub fn node_event_source_got_event(event: &Event) {
    info!(
        slog_scope::logger(),
        "node event source received event";
        "node_event" => format!("{event:#?}")
    );
}

pub fn node_event_source_eof(source: &NodeEventSource) {
    info!(
        slog_scope::logger(),
        "node event source end of file on control socket";
        "node_event_source" => format!("{source:#?}")
    );
}

pub fn ci_event_source_created(source: &CiEventSource) {
    info!(
        slog_scope::logger(),
        "created CI event source";
        "source" => format!("{source:#?}")
    );
}

pub fn ci_event_source_got_events(events: &[CiEvent]) {
    info!(
        slog_scope::logger(),
        "CI event source received events";
        "ci_events" => format!("{events:#?}")
    );
}

pub fn ci_event_source_disconnected() {
    info!(
        slog_scope::logger(),
        "CI event source received disconnection"
    );
}

pub fn ci_event_source_end() {
    info!(
        slog_scope::logger(),
        "CI event source was notified end of events"
    );
}

pub fn ci_event_source_eof(source: &CiEventSource) {
    info!(
        slog_scope::logger(),
        "CI event source end of file";
        "ci_event_source" => format!("{source:#?}")
    );
}

pub fn loaded_config(config: &Config) {
    debug!(slog_scope::logger(), "loaded configuration {config:#?}");
}

pub fn adapter_config(config: &Config) {
    debug!(slog_scope::logger(), "adapter configuration {config:#?}");
}

pub fn queueproc_start() {
    info!(
        slog_scope::logger(),
        "start thread to process events until a shutdown event"
    );
}

pub fn queueproc_end() {
    info!(slog_scope::logger(), "thread to process events ends");
}

pub fn queueproc_channel_disconnect() {
    info!(
        slog_scope::logger(),
        "event notification channel disconnected"
    );
}

pub fn queueproc_queue_length(len: usize) {
    trace!(
        slog_scope::logger(),
        "event queue length"; "length" => len);
}

pub fn queueproc_picked_event(id: &QueueId, event: &QueuedCiEvent) {
    info!(
        slog_scope::logger(),
        "picked event from queue: {id}: {event:#?}"
    );
}

pub fn queueproc_remove_event(id: &QueueId) {
    info!(slog_scope::logger(), "remove event from queue: {id}");
}

pub fn queueproc_action_run(rid: &RepoId, oid: &Oid) {
    info!(slog_scope::logger(), "Action: run: {rid} {oid}");
}

pub fn queueproc_action_shutdown() {
    info!(slog_scope::logger(), "Action: shutdown");
}

pub fn queueadd_start() {
    info!(
        slog_scope::logger(),
        "start thread to add events from node to event queue"
    );
}

pub fn queueadd_control_socket_close() {
    info!(
        slog_scope::logger(),
        "no more events from node control socket"
    );
}

pub fn queueadd_push_event(e: &CiEvent) {
    debug!(
        slog_scope::logger(),
        "insert broker event into queue: {e:?}"
    );
}

pub fn queueadd_end() {
    info!(slog_scope::logger(), "thread to process events ends");
}

pub fn pages_directory_unset() {
    warn!(
        slog_scope::logger(),
        "not writing HTML report pages as output directory has not been set"
    );
}

pub fn pages_interval(interval: Duration) {
    info!(
        slog_scope::logger(),
        "wait about {} seconds to update HTML report pages again",
        interval.as_secs()
    );
}

pub fn pages_disconnected() {
    info!(
        slog_scope::logger(),
        "page updater: run notification channel disconnected"
    );
}

pub fn pages_start() {
    info!(slog_scope::logger(), "start page updater thread");
}

pub fn pages_end() {
    info!(slog_scope::logger(), "end page updater thread");
}

pub fn event_disconnected() {
    info!(
        slog_scope::logger(),
        "connection to node control socket broke"
    );
}

pub fn event_end() {
    info!(
        slog_scope::logger(),
        "no more node events from control socket: iterator ended"
    );
}

pub fn broker_db(filename: &Path) {
    info!(
        slog_scope::logger(),
        "broker database: {}",
        filename.display()
    );
}

pub fn broker_start_run(trigger: &Request) {
    info!(slog_scope::logger(), "start CI run");
    debug!(slog_scope::logger(), "trigger event: {trigger:#?}");
}

pub fn broker_end_run(run: &Run) {
    info!(slog_scope::logger(), "Finish CI run");
    debug!(slog_scope::logger(), "finished CI run: {run:#?}");
}

pub fn adapter_no_first_response() {
    error!(slog_scope::logger(), "no first response message");
}

pub fn adapter_no_second_response() {
    error!(slog_scope::logger(), "no second response message");
}

pub fn adapter_too_many_responses() {
    error!(slog_scope::logger(), "too many response messages");
}

pub fn adapter_result(exit: Option<i32>, stderr: &str) {
    if let Some(exit) = exit {
        debug!(slog_scope::logger(), "adapter exit code"; "exit_code" => exit);
    } else {
        debug!(slog_scope::logger(), "adapter was terminated by signal");
    }
    for line in stderr.lines() {
        debug!(slog_scope::logger(), "adapter stderr"; "stderr" => line);
    }
}

pub fn adapter_did_not_exit_voluntarily() {
    warn!(slog_scope::logger(), "adapter did not exit voluntarily");
}

pub fn debug(msg: &str) {
    debug!(slog_scope::logger(), "{msg}");
}

pub fn debug2(msg: String) {
    debug!(slog_scope::logger(), "{msg}");
}

pub fn error(msg: &str, e: &impl std::error::Error) {
    error!(slog_scope::logger(), "{msg}: {e}");
    let mut e = e.source();
    while let Some(source) = e {
        error!(slog_scope::logger(), "caused by: {}", source);
        e = source.source();
    }
}
