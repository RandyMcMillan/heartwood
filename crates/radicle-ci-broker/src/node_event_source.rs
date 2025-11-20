//! Read node events from the local node.

use std::{fmt, path::PathBuf, time};

use radicle::{
    node::{Event, Handle},
    Profile,
};

use crate::logger;

/// Source of events from the local Radicle node.
pub struct NodeEventSource {
    profile_path: PathBuf,
    events: Box<dyn Iterator<Item = Result<Event, radicle::node::Error>>>,
}

impl NodeEventSource {
    /// Create a new source of node events, for a given Radicle
    /// profile.
    pub fn new(profile: &Profile) -> Result<Self, NodeEventError> {
        let socket = profile.socket();
        if !socket.exists() {
            return Err(NodeEventError::NoControlSocket(socket));
        }
        let node = radicle::Node::new(socket.clone());
        let source = match node.subscribe(time::Duration::MAX) {
            Ok(events) => Ok(Self {
                profile_path: profile.home.path().into(),
                events: Box::new(events.into_iter()),
            }),
            Err(err) => {
                logger::error("failed to subscribe to node events", &err);
                Err(NodeEventError::CannotSubscribe(socket.clone(), err))
            }
        }?;
        logger::node_event_source_created(&source);
        Ok(source)
    }

    /// Get the next node event from an event source, without
    /// filtering. This will block until there is an event, or until
    /// there will be no more events from this source, or there's an
    /// error.
    ///
    /// A closed or broken connection to the node is not an error,
    /// it's treated as end of file.
    pub fn node_event(&mut self) -> Result<Option<Event>, NodeEventError> {
        logger::debug("node_event: try to get an event");
        if let Some(event) = self.events.next() {
            match event {
                Ok(event) => {
                    logger::node_event_source_got_event(&event);
                    Ok(Some(event))
                }
                Err(radicle::node::Error::Io(err))
                    if err.kind() == std::io::ErrorKind::ConnectionReset =>
                {
                    logger::event_disconnected();
                    Ok(None)
                }
                Err(err) => {
                    logger::error("error reading event from node", &err);
                    Err(NodeEventError::Node(err))
                }
            }
        } else {
            logger::node_event_source_eof(self);
            Ok(None)
        }
    }
}

impl fmt::Debug for NodeEventSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NodeEventSource<path={}", self.profile_path.display())
    }
}

/// Possible errors from accessing the local Radicle node.
#[derive(Debug, thiserror::Error)]
pub enum NodeEventError {
    /// Regex compilation error.
    #[error("programming error in regular expression {0:?}")]
    Regex(&'static str, regex::Error),

    /// Node control socket does not exist.
    #[error("node control socket does not exist: {0}")]
    NoControlSocket(PathBuf),

    /// Can't subscribe to node events.
    #[error("failed to subscribe to node events on socket {0}")]
    CannotSubscribe(PathBuf, #[source] radicle::node::Error),

    /// Some error from getting an event from the node.
    #[error(transparent)]
    Node(#[from] radicle::node::Error),

    /// Connection to the node control socket broke.
    #[error("connection to the node control socket broke")]
    BrokenConnection,

    /// Some error from parsing a repository id.
    #[error(transparent)]
    Id(#[from] radicle::identity::IdError),

    /// Some error doing input/output.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// An error reading a filter file.
    #[error("failed to read filter file: {0}")]
    ReadFilterFile(PathBuf, #[source] std::io::Error),

    /// An error parsing JSON as filters, when read from a file.
    #[error("failed to parser filters file: {0}")]
    FiltersJsonFile(PathBuf, #[source] serde_json::Error),

    /// An error parsing YAML as filters, when read from a file.
    #[error("failed to parser filters file: {0}")]
    FiltersYamlFile(PathBuf, #[source] serde_yml::Error),

    /// An error parsing JSON as filters, from an in-memory string.
    #[error("failed to parser filters as JSON")]
    FiltersJsonString(#[source] serde_json::Error),

    /// An error parsing a Git object id as string into an Oid.
    #[error("failed to parse string as a Git object id: {0:?}")]
    ParseOid(String, #[source] radicle::git::raw::Error),
}
