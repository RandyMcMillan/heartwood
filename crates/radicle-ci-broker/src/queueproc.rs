//! Process events in the persistent event queue.

#![allow(clippy::result_large_err)]

use std::{
    sync::mpsc::RecvTimeoutError,
    thread::{spawn, JoinHandle},
};

use radicle::Profile;

use crate::{
    broker::{Broker, BrokerError},
    ci_event::{CiEvent, CiEventV1},
    db::{Db, DbError, QueueId, QueuedCiEvent},
    logger,
    msg::{MessageError, RequestBuilder},
    notif::{NotificationReceiver, NotificationSender},
};

#[derive(Default)]
pub struct QueueProcessorBuilder {
    db: Option<Db>,
    broker: Option<Broker>,
    events_rx: Option<NotificationReceiver>,
    run_tx: Option<NotificationSender>,
}

impl QueueProcessorBuilder {
    pub fn build(self) -> Result<QueueProcessor, QueueError> {
        Ok(QueueProcessor {
            db: self.db.ok_or(QueueError::Missing("db"))?,
            profile: Profile::load().map_err(QueueError::Profile)?,
            broker: self.broker.ok_or(QueueError::Missing("broker"))?,
            events_rx: self.events_rx.ok_or(QueueError::Missing("events_rx"))?,
            run_tx: self.run_tx.ok_or(QueueError::Missing("run_tx"))?,
        })
    }

    pub fn events_rx(mut self, rx: NotificationReceiver) -> Self {
        self.events_rx = Some(rx);
        self
    }

    pub fn run_tx(mut self, tx: NotificationSender) -> Self {
        self.run_tx = Some(tx);
        self
    }

    pub fn db(mut self, db: Db) -> Self {
        self.db = Some(db);
        self
    }

    pub fn broker(mut self, broker: Broker) -> Self {
        self.broker = Some(broker);
        self
    }
}

pub struct QueueProcessor {
    db: Db,
    profile: Profile,
    broker: Broker,
    events_rx: NotificationReceiver,
    run_tx: NotificationSender,
}

impl QueueProcessor {
    pub fn process_in_thread(mut self) -> JoinHandle<Result<(), QueueError>> {
        spawn(move || self.process_until_shutdown())
    }

    fn process_until_shutdown(&mut self) -> Result<(), QueueError> {
        logger::queueproc_start();
        let mut done = false;
        while !done {
            while let Some(qe) = self.pick_event()? {
                self.run_tx.notify()?;
                logger::queueproc_picked_event(qe.id(), &qe);
                done = self.process_event(qe.event())?;
                self.drop_event(qe.id())?;
                self.run_tx.notify()?;
            }
            match self.events_rx.wait_for_notification() {
                Ok(_) => {}
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    logger::queueproc_channel_disconnect();
                    done = true;
                }
            }
        }

        logger::queueproc_end();
        Ok(())
    }

    fn pick_event(&self) -> Result<Option<QueuedCiEvent>, QueueError> {
        let ids = self.db.queued_ci_events().map_err(QueueError::db)?;
        logger::queueproc_queue_length(ids.len());

        let mut queue = vec![];
        for id in ids.iter() {
            if let Some(qe) = self.db.get_queued_ci_event(id).map_err(QueueError::db)? {
                queue.push(qe);
            }
        }
        queue.sort_by_cached_key(|qe| qe.timestamp().to_string());

        if let Some(qe) = queue.first() {
            Ok(Some(qe.clone()))
        } else {
            Ok(None)
        }
    }

    fn process_event(&mut self, event: &CiEvent) -> Result<bool, QueueError> {
        match event {
            CiEvent::V1(CiEventV1::Shutdown) => {
                logger::queueproc_action_shutdown();
                Ok(true)
            }
            CiEvent::V1(CiEventV1::BranchCreated {
                from_node: _,
                repo,
                branch: _,
                tip,
            }) => {
                logger::queueproc_action_run(repo, tip);
                let trigger = RequestBuilder::default()
                    .profile(&self.profile)
                    .ci_event(event)
                    .build_trigger_from_ci_event()
                    .map_err(|e| QueueError::build_trigger(event, e))?;
                self.broker
                    .execute_ci(&trigger, &self.run_tx)
                    .map_err(QueueError::execute_ci)?;
                Ok(false)
            }
            CiEvent::V1(CiEventV1::BranchUpdated {
                from_node: _,
                repo,
                branch: _,
                tip,
                old_tip: _,
            }) => {
                logger::queueproc_action_run(repo, tip);
                let trigger = RequestBuilder::default()
                    .profile(&self.profile)
                    .ci_event(event)
                    .build_trigger_from_ci_event()
                    .map_err(|e| QueueError::build_trigger(event, e))?;
                self.broker
                    .execute_ci(&trigger, &self.run_tx)
                    .map_err(QueueError::execute_ci)?;
                Ok(false)
            }
            _ => unimplemented!("unknown CI event {event:#?}"),
        }
    }

    fn drop_event(&mut self, id: &QueueId) -> Result<(), QueueError> {
        logger::queueproc_remove_event(id);
        self.db.remove_queued_ci_event(id).map_err(QueueError::db)?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum QueueError {
    #[error("failed to load node profile")]
    Profile(#[source] radicle::profile::Error),

    #[error("programming error: QueueProcessorBuilder field {0} was not set")]
    Missing(&'static str),

    #[error("failed to use SQLite database")]
    Db(#[source] DbError),

    #[error("failed to create a trigger message from broker event {0:?}")]
    BuildTrigger(CiEvent, #[source] MessageError),

    #[error("failed to run CI")]
    ExecuteCi(#[source] BrokerError),

    #[error(transparent)]
    NotifyRun(#[from] crate::notif::NotificationError),
}

impl QueueError {
    fn db(e: DbError) -> Self {
        Self::Db(e)
    }

    fn build_trigger(event: &CiEvent, err: MessageError) -> Self {
        Self::BuildTrigger(event.clone(), err)
    }

    fn execute_ci(e: BrokerError) -> Self {
        Self::ExecuteCi(e)
    }
}
