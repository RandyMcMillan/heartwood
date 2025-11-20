//! Radicle CI broker.
//!
//! The broker triggers CI runs based on events emitted by its local
//! Radicle CI node. This crate allows listening to events from the
//! node, filter the events, and to communicate with a adapter spawned
//! as a child process.

#![deny(clippy::unwrap_used)]

pub mod adapter;
pub mod broker;
pub mod ci_event;
pub mod ci_event_source;
pub mod config;
pub mod db;
pub mod filter;
pub mod logger;
pub mod msg;
pub mod node_event_source;
pub mod notif;
pub mod pages;
pub mod queueadd;
pub mod queueproc;
pub mod run;
#[cfg(test)]
pub mod test;
pub mod util;
