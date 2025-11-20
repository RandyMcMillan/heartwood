use std::thread::{spawn, JoinHandle};

use radicle::Profile;

use crate::{
    ci_event::CiEvent,
    ci_event_source::{CiEventSource, CiEventSourceError},
    db::{Db, DbError},
    filter::EventFilter,
    logger,
    notif::NotificationSender,
};

#[derive(Default)]
pub struct QueueAdderBuilder {
    db: Option<Db>,
    filters: Option<Vec<EventFilter>>,
    events_tx: Option<NotificationSender>,
}

impl QueueAdderBuilder {
    pub fn build(self) -> Result<QueueAdder, AdderError> {
        Ok(QueueAdder {
            db: self.db.ok_or(AdderError::Missing("db"))?,
            filters: self.filters.ok_or(AdderError::Missing("filters"))?,
            events_tx: self.events_tx.ok_or(AdderError::Missing("events_tx"))?,
        })
    }

    pub fn events_tx(mut self, tx: NotificationSender) -> Self {
        self.events_tx = Some(tx);
        self
    }

    pub fn db(mut self, db: Db) -> Self {
        self.db = Some(db);
        self
    }

    pub fn filters(mut self, filters: &[EventFilter]) -> Self {
        self.filters = Some(filters.to_vec());
        self
    }
}

pub struct QueueAdder {
    filters: Vec<EventFilter>,
    db: Db,
    events_tx: NotificationSender,
}

impl QueueAdder {
    pub fn add_events_in_thread(self) -> JoinHandle<Result<(), AdderError>> {
        spawn(move || self.add_events())
    }

    pub fn add_events(&self) -> Result<(), AdderError> {
        logger::queueadd_start();

        let profile = Profile::load()?;

        let mut source = CiEventSource::new(&profile)?;

        // This loop ends when there's an error, e.g., failure to read an
        // event from the node.
        'event_loop: loop {
            let events = source.event();
            logger::debug2(format!("queueadd: events={events:?}"));
            match events {
                Err(e) => {
                    logger::queueadd_control_socket_close();
                    return Err(e.into());
                }
                Ok(None) => {
                    break 'event_loop;
                }
                Ok(Some(events)) => {
                    for e in events {
                        for filter in self.filters.iter() {
                            if filter.allows(&e) {
                                logger::queueadd_push_event(&e);
                                self.push_event(e.clone())?;
                            }
                        }
                    }
                }
            }
        }

        logger::queueadd_end();
        Ok(())
    }

    fn push_event(&self, e: CiEvent) -> Result<(), AdderError> {
        self.db.push_queued_ci_event(e)?;
        self.events_tx.notify().map_err(|_| AdderError::Send)?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AdderError {
    #[error("programming error: QueueAdderBuilder field {0} was not set")]
    Missing(&'static str),

    #[error(transparent)]
    Profile(#[from] radicle::profile::Error),

    #[error(transparent)]
    CiEvent(#[from] CiEventSourceError),

    #[error(transparent)]
    Db(#[from] DbError),

    #[error("failed to notify other thread about database change")]
    Send,
}
