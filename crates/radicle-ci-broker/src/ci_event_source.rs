use std::fmt;

use radicle::Profile;

use crate::{
    ci_event::CiEvent,
    logger,
    node_event_source::{NodeEventError, NodeEventSource},
};

pub struct CiEventSource {
    source: NodeEventSource,
}

impl CiEventSource {
    pub fn new(profile: &Profile) -> Result<Self, CiEventSourceError> {
        let source = Self {
            source: NodeEventSource::new(profile).map_err(CiEventSourceError::Subscribe)?,
        };
        logger::ci_event_source_created(&source);
        Ok(source)
    }

    pub fn event(&mut self) -> Result<Option<Vec<CiEvent>>, CiEventSourceError> {
        let result = self.source.node_event();
        logger::debug2(format!("ci_event_source: result={result:?}"));
        match result {
            Err(err) if matches!(err, NodeEventError::BrokenConnection) => {
                logger::ci_event_source_disconnected();
                Err(CiEventSourceError::BrokenConnection(err))
            }
            Err(err) => {
                logger::error("error reading event from node", &err);
                Err(CiEventSourceError::NodeEventError(err))
            }
            Ok(None) => {
                logger::ci_event_source_end();
                Ok(None)
            }
            Ok(Some(event)) => {
                let ci_events =
                    CiEvent::from_node_event(&event).map_err(CiEventSourceError::CiEvent)?;
                logger::ci_event_source_got_events(&ci_events);
                Ok(Some(ci_events))
            }
        }
    }
}

impl fmt::Debug for CiEventSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CiEventSource<path={:?}", self.source)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CiEventSourceError {
    #[error("failed to subscribe to node events")]
    Subscribe(#[source] crate::node_event_source::NodeEventError),

    #[error("connection to node control socket broke")]
    BrokenConnection(#[source] crate::node_event_source::NodeEventError),

    #[error("failed to read event from node")]
    NodeEventError(#[source] crate::node_event_source::NodeEventError),

    #[error("failed to create CI events from node event")]
    CiEvent(#[source] crate::ci_event::CiEventError),
}
