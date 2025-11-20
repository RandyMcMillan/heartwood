//! Notification channel between threads.

use std::{
    sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender},
    time::Duration,
};

// Timeout when receiving notification about a new event.
const EVENT_RECV_TIMEOUT: Duration = Duration::from_secs(1);

// Maximum interval between update HTML report pages.
const UPDATE_IMTERVAL: Duration = Duration::from_secs(1);

/// Channel endpoint for sending notifications.
pub struct NotificationSender {
    sender: Sender<()>,
}

impl NotificationSender {
    fn new(sender: Sender<()>) -> Self {
        Self { sender }
    }

    pub fn notify(&self) -> Result<(), NotificationError> {
        self.sender.send(()).map_err(NotificationError::Send)
    }
}

/// Channel endpoint for receiving notifications.
pub struct NotificationReceiver {
    receiver: Receiver<()>,
    max_wait: Duration,
}

impl NotificationReceiver {
    fn new(receiver: Receiver<()>, max_wait: Duration) -> Self {
        Self { receiver, max_wait }
    }

    pub fn wait_for_notification(&self) -> Result<(), RecvTimeoutError> {
        self.receiver.recv_timeout(self.max_wait)
    }
}

/// Notification channel.
///
/// The notification channel allows one thread to notify another that
/// the other thread has some work to do. The notification carries no
/// other information: the receiver is supposed to know where it can
/// get whatever data it needs to what it needs to do.
///
/// The point of this is to make sure threads in the CI broker
/// exchange data only via the database, where it is persistent.
pub struct NotificationChannel {
    tx: Option<NotificationSender>,
    rx: Option<NotificationReceiver>,
}

impl NotificationChannel {
    /// Construct a channel for notifying about new events.
    pub fn new_event() -> Self {
        Self::new(EVENT_RECV_TIMEOUT)
    }

    /// Construct a channel for notifying about new CI runs.
    pub fn new_run() -> Self {
        Self::new(UPDATE_IMTERVAL)
    }

    fn new(max_wait: Duration) -> Self {
        let (tx, rx) = channel();
        Self {
            tx: Some(NotificationSender::new(tx)),
            rx: Some(NotificationReceiver::new(rx, max_wait)),
        }
    }
}

impl NotificationChannel {
    /// Return the transmit endpoint of the notification channel. This
    /// can only be called once.
    pub fn tx(&mut self) -> Result<NotificationSender, NotificationError> {
        self.tx.take().ok_or(NotificationError::Sender)
    }

    /// Return the receive endpoint of the notification channel. This
    /// can only be called once.
    pub fn rx(&mut self) -> Result<NotificationReceiver, NotificationError> {
        self.rx.take().ok_or(NotificationError::Receiver)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum NotificationError {
    #[error("receiver end point already extracted")]
    Receiver,

    #[error("sender end point already extracted")]
    Sender,

    #[error("error sending to channel")]
    Send(#[source] std::sync::mpsc::SendError<()>),
}
