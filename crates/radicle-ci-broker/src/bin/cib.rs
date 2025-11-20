//! The CI broker.

#![allow(clippy::result_large_err)]

use std::{
    fs::write,
    path::{Path, PathBuf},
    process::exit,
};

use clap::Parser;

use radicle::Profile;

use radicle_ci_broker::{
    adapter::Adapter,
    broker::{Broker, BrokerError},
    config::{Config, ConfigError},
    db::{Db, DbError},
    logger::{self, LogLevel},
    notif::{NotificationChannel, NotificationError},
    pages::StatusPage,
    queueadd::{AdderError, QueueAdderBuilder},
    queueproc::{QueueError, QueueProcessorBuilder},
};

fn main() {
    let logger = logger::open();

    if let Err(e) = fallible_main(&logger) {
        logger::error("ERROR", &e);
        logger::end_cib_in_error();
        exit(1);
    }
    logger::end_cib_successfully();
}

fn fallible_main(logger: &logger::Logger) -> Result<(), CibError> {
    let args = Args::parse();
    logger.set_minimum_level(args.minimum_log_level());

    // We only log this after setting the minimum log level from the
    // command line.
    logger::start_cib();

    let config = Config::load(&args.config).map_err(|e| CibError::read_config(&args.config, e))?;
    logger::loaded_config(&config);

    args.run(&config)?;

    Ok(())
}

#[derive(Debug, Parser)]
#[command(version)]
struct Args {
    #[clap(long)]
    config: PathBuf,

    #[clap(long, value_enum)]
    log_level: Option<logger::LogLevel>,

    #[clap(subcommand)]
    cmd: Cmd,
}

impl Args {
    fn run(&self, config: &Config) -> Result<(), CibError> {
        match &self.cmd {
            Cmd::Config(x) => x.run(self, config)?,
            Cmd::Insert(x) => x.run(self, config)?,
            Cmd::Queued(x) => x.run(self, config)?,
            Cmd::ProcessEvents(x) => x.run(self, config)?,
        }
        Ok(())
    }

    fn open_db(&self, config: &Config) -> Result<Db, CibError> {
        let db = Db::new(&config.db).map_err(CibError::db)?;
        let events = db.queued_events().map_err(CibError::Db)?;
        if events.is_empty() {
            Ok(db)
        } else {
            Err(CibError::UnprocessedBrokerEvents)
        }
    }

    fn minimum_log_level(&self) -> LogLevel {
        self.log_level.unwrap_or(LogLevel::Trace)
    }
}

#[derive(Debug, Parser)]
enum Cmd {
    Config(ConfigCmd),
    Insert(InsertCmd),
    Queued(QueuedCmd),
    ProcessEvents(ProcessEventsCmd),
}

#[derive(Debug, Parser)]
struct ConfigCmd {
    #[clap(long)]
    output: Option<PathBuf>,
}

impl ConfigCmd {
    fn run(&self, args: &Args, config: &Config) -> Result<(), CibError> {
        let json = config
            .to_json()
            .map_err(|e| CibError::to_json(&args.config, e))?;

        if let Some(output) = &self.output {
            write(output, json.as_bytes()).map_err(|e| CibError::write_config(output, e))?;
        } else {
            println!("{json}");
        }

        Ok(())
    }
}

#[derive(Debug, Parser)]
struct InsertCmd {}

impl InsertCmd {
    fn run(&self, args: &Args, config: &Config) -> Result<(), CibError> {
        let mut events_notification = NotificationChannel::new_event();
        let adder = QueueAdderBuilder::default()
            .events_tx(events_notification.tx().map_err(CibError::notification)?)
            .db(args.open_db(config)?)
            .filters(&config.filters)
            .build()
            .map_err(CibError::QueueAdder)?;
        let thread = adder.add_events_in_thread();
        thread
            .join()
            .expect("wait for thread to finish")
            .map_err(CibError::add_events)?;
        Ok(())
    }
}

#[derive(Debug, Parser)]
struct QueuedCmd {}

impl QueuedCmd {
    fn run(&self, args: &Args, config: &Config) -> Result<(), CibError> {
        let profile = Profile::load().map_err(CibError::profile)?;

        let mut broker = Broker::new(config.db()).map_err(CibError::new_broker)?;
        let spec =
            config
                .adapter(&config.default_adapter)
                .ok_or(CibError::UnknownDefaultAdapter(
                    config.default_adapter.clone(),
                ))?;
        let adapter = Adapter::new(&spec.command)
            .with_environment(spec.envs())
            .with_environment(spec.sensitive_envs());
        logger::adapter_config(config);
        broker.set_default_adapter(&adapter);

        let mut event_notifications = NotificationChannel::new_event();
        event_notifications
            .tx()
            .map_err(CibError::notification)?
            .notify()
            .ok();

        let mut run_notifications = NotificationChannel::new_run();

        let db = args.open_db(config)?;
        let processor = QueueProcessorBuilder::default()
            .events_rx(event_notifications.rx().map_err(CibError::notification)?)
            .run_tx(run_notifications.tx().map_err(CibError::notification)?)
            .db(db)
            .broker(broker)
            .build()
            .map_err(CibError::process_queue)?;
        let thread = processor.process_in_thread();
        thread
            .join()
            .expect("wait for thread to finish")
            .map_err(CibError::process_queue)?;

        let db = args.open_db(config)?;
        let mut page = StatusPage::default();
        if let Some(dirname) = &config.report_dir {
            page.set_output_dir(dirname);
        }
        let page_updater = page.update_in_thread(
            run_notifications.rx().map_err(CibError::notification)?,
            profile,
            db,
            true,
        );
        page_updater
            .join()
            .expect("wait for page updater thread to finish")
            .expect("page updater thread succeeded");

        Ok(())
    }
}

#[derive(Debug, Parser)]
struct ProcessEventsCmd {}

impl ProcessEventsCmd {
    fn run(&self, args: &Args, config: &Config) -> Result<(), CibError> {
        let mut events_notification = NotificationChannel::new_event();
        let mut run_notification = NotificationChannel::new_run();

        let adder = QueueAdderBuilder::default()
            .events_tx(events_notification.tx().map_err(CibError::notification)?)
            .db(args.open_db(config)?)
            .filters(&config.filters)
            .build()
            .map_err(CibError::QueueAdder)?;
        adder.add_events_in_thread();

        let profile = Profile::load().map_err(CibError::profile)?;

        let db = args.open_db(config)?;

        let mut page = StatusPage::default();
        if let Some(dirname) = &config.report_dir {
            page.set_output_dir(dirname);
        }
        let page_updater = page.update_in_thread(
            run_notification.rx().map_err(CibError::notification)?,
            profile,
            db,
            false,
        );

        let mut broker = Broker::new(config.db()).map_err(CibError::new_broker)?;
        let spec =
            config
                .adapter(&config.default_adapter)
                .ok_or(CibError::UnknownDefaultAdapter(
                    config.default_adapter.clone(),
                ))?;
        let adapter = Adapter::new(&spec.command)
            .with_environment(spec.envs())
            .with_environment(spec.sensitive_envs());
        logger::adapter_config(config);
        broker.set_default_adapter(&adapter);

        let processor = QueueProcessorBuilder::default()
            .events_rx(events_notification.rx().map_err(CibError::notification)?)
            .run_tx(run_notification.tx().map_err(CibError::notification)?)
            .db(args.open_db(config)?)
            .broker(broker)
            .build()
            .map_err(CibError::process_queue)?;
        let processor = processor.process_in_thread();
        processor
            .join()
            .expect("wait for processor thread to finish")
            .map_err(CibError::process_queue)?;

        // The page updating thread ends when the channel for run
        // notifications is closed by the processor thread ending.
        page_updater
            .join()
            .expect("wait for page updater thread to finish")
            .expect("page updater thread succeeded");

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
enum CibError {
    #[error("failed to read configuration file {0}")]
    ReadConfig(PathBuf, #[source] ConfigError),

    #[error("failed to write config as JSON to file {0}")]
    WriteConfig(PathBuf, #[source] std::io::Error),

    #[error("failed to convert config as JSON")]
    ToJson(PathBuf, #[source] ConfigError),

    #[error("failed to look up node profile")]
    Profile(#[source] radicle::profile::Error),

    #[error("failed to use SQLite database")]
    Db(#[source] DbError),

    #[error("database has unprocessed broker events")]
    UnprocessedBrokerEvents,

    #[error("failed create broker data type")]
    NewBroker(#[source] BrokerError),

    #[error("failed to add node events into queue")]
    QueueAdder(#[source] AdderError),

    #[error("failed to process events from queue")]
    ProcessQueue(#[source] QueueError),

    #[error("failed to add events to queue")]
    AddEvents(#[source] AdderError),

    #[error("default adapter is not in list of adapters")]
    UnknownDefaultAdapter(String),

    #[error("programming error: failed to set up inter-thread notification channel")]
    Notification(#[source] NotificationError),
}

impl CibError {
    fn read_config(filename: &Path, e: ConfigError) -> Self {
        Self::ReadConfig(filename.into(), e)
    }

    fn write_config(filename: &Path, e: std::io::Error) -> Self {
        Self::WriteConfig(filename.into(), e)
    }

    fn to_json(filename: &Path, e: ConfigError) -> Self {
        Self::ToJson(filename.into(), e)
    }

    fn profile(e: radicle::profile::Error) -> Self {
        Self::Profile(e)
    }

    fn db(e: DbError) -> Self {
        Self::Db(e)
    }

    fn new_broker(e: BrokerError) -> Self {
        Self::NewBroker(e)
    }

    fn process_queue(e: QueueError) -> Self {
        Self::ProcessQueue(e)
    }

    fn add_events(e: AdderError) -> Self {
        Self::AddEvents(e)
    }

    fn notification(e: NotificationError) -> Self {
        Self::Notification(e)
    }
}
