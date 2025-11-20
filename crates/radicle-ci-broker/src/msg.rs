//! Messages for communicating with the adapter.
//!
//! The broker spawns an adapter child process, and sends it a request
//! via the child's stdin. The child sends responses via its stdout,
//! which the broker reads and processes. These messages are
//! represented using the types in this module.
//!
//! The types in this module are meant to be useful for anyone writing
//! a Radicle CI adapter.

#![deny(missing_docs)]

use std::{
    fmt,
    hash::{Hash, Hasher},
    io::{BufRead, BufReader, Read, Write},
};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use radicle::{
    git::Oid,
    prelude::{NodeId, RepoId},
};
use radicle::{
    identity::Did,
    node::{Alias, AliasStore},
    patch::{self, RevisionId},
    storage::{git::paths, ReadRepository, ReadStorage},
    Profile,
};

use crate::ci_event::{CiEvent, CiEventV1};

// This gets put into every [`Request`] message so the adapter can
// detect its getting a message it knows how to handle.
const PROTOCOL_VERSION: usize = 1;

/// The type of a run identifier. For maximum generality, this is a
/// string rather than an integer.
///
/// # Example
/// ```rust
/// use radicle_ci_broker::msg::RunId;
/// let id = RunId::from("abracadabra");
/// println!("{}", id.to_string());
/// ```
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct RunId {
    id: String,
}

impl Default for RunId {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
        }
    }
}

impl Hash for RunId {
    fn hash<H: Hasher>(&self, h: &mut H) {
        self.id.hash(h);
    }
}

impl From<&str> for RunId {
    fn from(id: &str) -> Self {
        Self { id: id.into() }
    }
}

impl fmt::Display for RunId {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.id)
    }
}

impl RunId {
    /// Return representation of identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.id
    }
}

/// The result of a CI run.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum RunResult {
    /// CI run was successful.
    Success,

    /// CI run failed.
    Failure,
}

impl fmt::Display for RunResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Self::Failure => write!(f, "failure"),
            Self::Success => write!(f, "success"),
        }
    }
}

/// Build a [`Request`].
#[derive(Debug, Default)]
pub struct RequestBuilder<'a> {
    profile: Option<&'a Profile>,
    ci_event: Option<&'a CiEvent>,
}

impl<'a> RequestBuilder<'a> {
    /// Set the node profile to use.
    pub fn profile(mut self, profile: &'a Profile) -> Self {
        self.profile = Some(profile);
        self
    }

    /// Set the CI event to use.
    pub fn ci_event(mut self, event: &'a CiEvent) -> Self {
        self.ci_event = Some(event);
        self
    }

    /// Create a [`Request::Trigger``] message from a [`crate::ci_event::Civet`].
    pub fn build_trigger_from_ci_event(self) -> Result<Request, MessageError> {
        let profile = self.profile.ok_or(MessageError::NoProfile)?;

        match self.ci_event {
            None => Err(MessageError::CiEventNotSet),
            Some(CiEvent::V1(CiEventV1::BranchCreated {
                from_node,
                repo,
                branch,
                tip,
            })) => {
                let rad_repo = profile.storage.repository(*repo)?;
                let project_info = rad_repo.project()?;

                let common = EventCommonFields {
                    version: PROTOCOL_VERSION,
                    event_type: EventType::Push,
                    repository: Repository {
                        id: *repo,
                        name: project_info.name().to_string(),
                        description: project_info.description().to_string(),
                        private: !rad_repo.identity()?.visibility.is_public(),
                        default_branch: project_info.default_branch().to_string(),
                        delegates: rad_repo.delegates()?.iter().copied().collect(),
                    },
                };

                let did = Did::from(*from_node);
                let pusher = did_to_author(profile, &did)?;

                let push = PushEvent {
                    pusher,
                    before: *tip, // Branch created: we only use the tip
                    after: *tip,
                    branch: push_branch(branch),
                    commits: vec![*tip], // Branch created, only use tip.
                };
                Ok(Request::Trigger {
                    common,
                    push: Some(push),
                    patch: None,
                })
            }
            Some(CiEvent::V1(CiEventV1::BranchUpdated {
                from_node,
                repo,
                branch,
                tip,
                old_tip,
            })) => {
                let rad_repo = profile.storage.repository(*repo)?;
                let git_repo =
                    radicle_surf::Repository::open(paths::repository(&profile.storage, repo))?;
                let project_info = rad_repo.project()?;

                let common = EventCommonFields {
                    version: PROTOCOL_VERSION,
                    event_type: EventType::Push,
                    repository: Repository {
                        id: *repo,
                        name: project_info.name().to_string(),
                        description: project_info.description().to_string(),
                        private: !rad_repo.identity()?.visibility.is_public(),
                        default_branch: project_info.default_branch().to_string(),
                        delegates: rad_repo.delegates()?.iter().copied().collect(),
                    },
                };

                let did = Did::from(*from_node);
                let pusher = did_to_author(profile, &did)?;

                let mut commits: Vec<Oid> = git_repo
                    .history(tip)?
                    .take_while(|c| {
                        if let Ok(c) = c {
                            c.id != *old_tip
                        } else {
                            false
                        }
                    })
                    .map(|r| r.map(|c| c.id))
                    .collect::<Result<Vec<Oid>, _>>()?;
                if commits.is_empty() {
                    commits = vec![*old_tip];
                }

                let push = PushEvent {
                    pusher,
                    before: *tip, // Branch created: we only use the tip
                    after: *tip,
                    branch: push_branch(branch),
                    commits,
                };

                Ok(Request::Trigger {
                    common,
                    push: Some(push),
                    patch: None,
                })
            }
            Some(CiEvent::V1(CiEventV1::PatchCreated {
                from_node,
                repo,
                patch: patch_id,
                new_tip,
            })) => {
                let rad_repo = profile.storage.repository(*repo)?;
                let git_repo =
                    radicle_surf::Repository::open(paths::repository(&profile.storage, repo))?;
                let project_info = rad_repo.project()?;

                let common = EventCommonFields {
                    version: PROTOCOL_VERSION,
                    event_type: EventType::Patch,
                    repository: Repository {
                        id: *repo,
                        name: project_info.name().to_string(),
                        description: project_info.description().to_string(),
                        private: !rad_repo.identity()?.visibility.is_public(),
                        default_branch: project_info.default_branch().to_string(),
                        delegates: rad_repo.delegates()?.iter().copied().collect(),
                    },
                };

                let did = Did::from(*from_node);
                let author = did_to_author(profile, &did)?;

                let patch_cob = patch::Patches::open(&rad_repo)?
                    .get(patch_id)?
                    .ok_or(MessageError::Trigger)?;

                let revisions: Vec<Revision> = patch_cob
                    .revisions()
                    .map(|(rid, r)| {
                        Ok::<Revision, MessageError>(Revision {
                            id: rid.into(),
                            author: did_to_author(profile, r.author().id())?,
                            description: r.description().to_string(),
                            base: *r.base(),
                            oid: r.head(),
                            timestamp: r.timestamp().as_secs(),
                        })
                    })
                    .collect::<Result<Vec<Revision>, MessageError>>()?;
                let patch_author_pk = radicle::crypto::PublicKey::from(author.id);
                let patch_latest_revision = patch_cob
                    .latest_by(&patch_author_pk)
                    .ok_or(MessageError::Trigger)?;
                let patch_base = patch_latest_revision.1.base();
                let commits: Vec<Oid> = git_repo
                    .history(*new_tip)?
                    .take_while(|c| {
                        if let Ok(c) = c {
                            c.id != *patch_base
                        } else {
                            false
                        }
                    })
                    .map(|r| r.map(|c| c.id))
                    .collect::<Result<Vec<Oid>, _>>()?;

                let patch = Patch {
                    id: **patch_id,
                    author,
                    title: patch_cob.title().to_string(),
                    state: State {
                        status: patch_cob.state().to_string(),
                        conflicts: match patch_cob.state() {
                            patch::State::Open { conflicts, .. } => conflicts.to_vec(),
                            _ => vec![],
                        },
                    },
                    before: *patch_base,
                    after: *new_tip,
                    commits,
                    target: patch_cob.target().head(&rad_repo)?,
                    labels: patch_cob.labels().map(|l| l.name().to_string()).collect(),
                    assignees: patch_cob.assignees().collect(),
                    revisions,
                };

                Ok(Request::Trigger {
                    common,
                    push: None,
                    patch: Some(PatchEvent {
                        action: PatchAction::Created,
                        patch,
                    }),
                })
            }
            Some(CiEvent::V1(CiEventV1::PatchUpdated {
                from_node,
                repo,
                patch: patch_id,
                new_tip,
            })) => {
                let rad_repo = profile.storage.repository(*repo)?;
                let git_repo =
                    radicle_surf::Repository::open(paths::repository(&profile.storage, repo))?;
                let project_info = rad_repo.project()?;

                let common = EventCommonFields {
                    version: PROTOCOL_VERSION,
                    event_type: EventType::Patch,
                    repository: Repository {
                        id: *repo,
                        name: project_info.name().to_string(),
                        description: project_info.description().to_string(),
                        private: !rad_repo.identity()?.visibility.is_public(),
                        default_branch: project_info.default_branch().to_string(),
                        delegates: rad_repo.delegates()?.iter().copied().collect(),
                    },
                };

                let did = Did::from(*from_node);
                let author = did_to_author(profile, &did)?;

                let patch_cob = patch::Patches::open(&rad_repo)?
                    .get(patch_id)?
                    .ok_or(MessageError::Trigger)?;

                let revisions: Vec<Revision> = patch_cob
                    .revisions()
                    .map(|(rid, r)| {
                        Ok::<Revision, MessageError>(Revision {
                            id: rid.into(),
                            author: did_to_author(profile, r.author().id())?,
                            description: r.description().to_string(),
                            base: *r.base(),
                            oid: r.head(),
                            timestamp: r.timestamp().as_secs(),
                        })
                    })
                    .collect::<Result<Vec<Revision>, MessageError>>()?;
                let patch_author_pk = radicle::crypto::PublicKey::from(author.id);
                let patch_latest_revision = patch_cob
                    .latest_by(&patch_author_pk)
                    .ok_or(MessageError::Trigger)?;
                let patch_base = patch_latest_revision.1.base();
                let commits: Vec<Oid> = git_repo
                    .history(*new_tip)?
                    .take_while(|c| {
                        if let Ok(c) = c {
                            c.id != *patch_base
                        } else {
                            false
                        }
                    })
                    .map(|r| r.map(|c| c.id))
                    .collect::<Result<Vec<Oid>, _>>()?;

                let patch = Patch {
                    id: **patch_id,
                    author,
                    title: patch_cob.title().to_string(),
                    state: State {
                        status: patch_cob.state().to_string(),
                        conflicts: match patch_cob.state() {
                            patch::State::Open { conflicts, .. } => conflicts.to_vec(),
                            _ => vec![],
                        },
                    },
                    before: *patch_base,
                    after: *new_tip,
                    commits,
                    target: patch_cob.target().head(&rad_repo)?,
                    labels: patch_cob.labels().map(|l| l.name().to_string()).collect(),
                    assignees: patch_cob.assignees().collect(),
                    revisions,
                };

                Ok(Request::Trigger {
                    common,
                    push: None,
                    patch: Some(PatchEvent {
                        action: PatchAction::Updated,
                        patch,
                    }),
                })
            }
            Some(event) => Err(MessageError::UnknownCiEvent(event.clone())),
        }
    }
}

/// A request message sent by the broker to its adapter child process.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "request")]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Request {
    /// Trigger a run.
    Trigger {
        /// Common fields for all message variants.
        #[serde(flatten)]
        common: EventCommonFields,

        /// The push event, if any.
        #[serde(flatten)]
        push: Option<PushEvent>,

        /// The patch event, if any.
        #[serde(flatten)]
        patch: Option<PatchEvent>,
    },
}

impl Request {
    /// Repository that the event concerns.
    pub fn repo(&self) -> RepoId {
        match self {
            Self::Trigger {
                common,
                push: _,
                patch: _,
            } => common.repository.id,
        }
    }

    /// Return the commit the event concerns. In other words, the
    /// commit that CI should run against.
    pub fn commit(&self) -> Result<Oid, MessageError> {
        match self {
            Self::Trigger {
                common: _,
                push,
                patch,
            } => {
                if let Some(push) = push {
                    if let Some(oid) = push.commits.first() {
                        Ok(*oid)
                    } else {
                        Err(MessageError::NoCommits)
                    }
                } else if let Some(patch) = patch {
                    if let Some(oid) = patch.patch.commits.first() {
                        Ok(*oid)
                    } else {
                        Err(MessageError::NoCommits)
                    }
                } else {
                    Err(MessageError::UnknownRequest)
                }
            }
        }
    }

    /// Serialize the request as a single-line JSON, including the
    /// newline. This is meant for the broker to use.
    pub fn to_writer<W: Write>(&self, mut writer: W) -> Result<(), MessageError> {
        let mut line = serde_json::to_string(&self).map_err(MessageError::SerializeRequest)?;
        line.push('\n');
        writer
            .write(line.as_bytes())
            .map_err(MessageError::WriteRequest)?;
        Ok(())
    }

    /// Read a request from a reader. This is meant for the adapter to
    /// use.
    pub fn from_reader<R: Read>(reader: R) -> Result<Self, MessageError> {
        let mut line = String::new();
        let mut r = BufReader::new(reader);
        r.read_line(&mut line).map_err(MessageError::ReadLine)?;
        let req: Self =
            serde_json::from_slice(line.as_bytes()).map_err(MessageError::DeserializeRequest)?;
        Ok(req)
    }

    /// Parse a request from a string. This is meant for tests to use.
    pub fn try_from_str(s: &str) -> Result<Self, MessageError> {
        let req: Self =
            serde_json::from_slice(s.as_bytes()).map_err(MessageError::DeserializeRequest)?;
        Ok(req)
    }
}

fn did_to_author(profile: &Profile, did: &Did) -> Result<Author, MessageError> {
    let alias = profile.aliases().alias(did);
    Ok(Author { id: *did, alias })
}

impl fmt::Display for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            serde_json::to_string(&self).map_err(|_| fmt::Error)?
        )
    }
}

/// Type of event.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EventType {
    /// A push event to a branch.
    Push,

    /// A new or changed patch.
    Patch,
}

/// Common fields in all variations of a [`Request`] message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventCommonFields {
    /// Version of the request message.
    pub version: usize,

    /// The type of the event.
    pub event_type: EventType,

    /// The repository the event is related to.
    pub repository: Repository,
}

/// A push event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushEvent {
    /// The author of the change.
    pub pusher: Author,

    /// The commit on which the change is based.
    pub before: Oid,

    /// FIXME
    pub after: Oid,

    /// The branch where the push occurred.
    pub branch: String,

    /// The commits in the change.
    pub commits: Vec<Oid>,
}

/// An event related to a Radicle patch object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchEvent {
    /// What action has happened to the patch.
    pub action: PatchAction,

    /// Metadata about the patch.
    pub patch: Patch,
}

/// What action has happened to the patch?
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatchAction {
    /// Patch has been created.
    Created,

    /// Patch has been updated.
    Updated,
}

#[cfg(test)]
impl PatchAction {
    fn as_str(&self) -> &str {
        match self {
            Self::Created => "created",
            Self::Updated => "updated",
        }
    }
}

impl TryFrom<&str> for PatchAction {
    type Error = MessageError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "created" => Ok(Self::Created),
            "updated" => Ok(Self::Updated),
            _ => Err(Self::Error::UnknownPatchAction(value.into())),
        }
    }
}

/// Fields in a [`Request`] message describing the repository
/// concerned.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    /// The unique repository id.
    pub id: RepoId,

    /// The name of the repository.
    pub name: String,

    /// A description of the repository.
    pub description: String,

    /// Is it a private repository?
    pub private: bool,

    /// The default branch in the repository: the branch that gets
    /// updated when a change is merged.
    pub default_branch: String,

    /// The delegates of the repository: those who can actually merge
    /// the change.
    pub delegates: Vec<Did>,
}

/// Fields describing the author of a change.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Author {
    /// The DID of the author. This is guaranteed to be unique.
    pub id: Did,

    /// The alias, or name, of the author. This need not be unique.
    pub alias: Option<Alias>,
}

impl std::fmt::Display for Author {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)?;
        if let Some(alias) = &self.alias {
            write!(f, " ({})", alias)?;
        }
        Ok(())
    }
}

/// The state of a patch.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct State {
    /// State of the patch.
    pub status: String,

    /// FIXME.
    pub conflicts: Vec<(RevisionId, Oid)>,
}

/// Revision of a patch.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Revision {
    /// FIXME.
    pub id: Oid,

    /// Author of the revision.
    pub author: Author,

    /// Description of the revision.
    pub description: String,

    /// Base commit on which the revision of the patch should be
    /// applied.
    pub base: Oid,

    /// FIXME.
    pub oid: Oid,

    /// Time stamp of the revision.
    pub timestamp: u64,
}

impl std::fmt::Display for Revision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}

/// Metadata about a Radicle patch.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Patch {
    /// The patch id.
    pub id: Oid,

    /// The author of the patch.
    pub author: Author,

    /// The title of the patch.
    pub title: String,

    /// The state of the patch.
    pub state: State,

    /// The commit preceding the patch.
    pub before: Oid,

    /// FIXME.
    pub after: Oid,

    /// The list of commits in the patch.
    pub commits: Vec<Oid>,

    /// FIXME.
    pub target: Oid,

    /// Labels assigned to the patch.
    pub labels: Vec<String>,

    /// Who're in charge of the patch.
    pub assignees: Vec<Did>,

    /// List of revisions of the patch.
    pub revisions: Vec<Revision>,
}

/// A response message from the adapter child process to the broker.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "response")]
pub enum Response {
    /// A CI run has been triggered.
    Triggered {
        /// The identifier for the CI run assigned by the adapter.
        run_id: RunId,

        /// Optional informational URL for the run.
        info_url: Option<String>,
    },

    /// A CI run has finished.
    Finished {
        /// The result of a CI run.
        result: RunResult,
    },
}

impl Response {
    /// Create a `Response::Triggered` message without an info URL.
    pub fn triggered(run_id: RunId) -> Self {
        Self::Triggered {
            run_id,
            info_url: None,
        }
    }

    /// Create a `Response::Triggered` message with an info URL.
    pub fn triggered_with_url(run_id: RunId, url: &str) -> Self {
        Self::Triggered {
            run_id,
            info_url: Some(url.into()),
        }
    }

    /// Create a `Response::Finished` message.
    pub fn finished(result: RunResult) -> Self {
        Self::Finished { result }
    }

    /// Does the message indicate a result for the CI run?
    pub fn result(&self) -> Option<&RunResult> {
        if let Self::Finished { result } = self {
            Some(result)
        } else {
            None
        }
    }

    /// Serialize a response as a single-line JSON, including the
    /// newline. This is meant for the adapter to use.
    pub fn to_writer<W: Write>(&self, mut writer: W) -> Result<(), MessageError> {
        let mut line = serde_json::to_string(&self).map_err(MessageError::SerializeResponse)?;
        line.push('\n');
        writer
            .write(line.as_bytes())
            .map_err(MessageError::WriteResponse)?;
        Ok(())
    }

    /// Read a response from a reader. This is meant for the broker to
    /// use.
    pub fn from_reader<R: Read + BufRead>(reader: &mut R) -> Result<Option<Self>, MessageError> {
        let mut line = String::new();
        let mut r = BufReader::new(reader);
        let n = r.read_line(&mut line).map_err(MessageError::ReadLine)?;
        if n == 0 {
            // Child's stdout was closed.
            Ok(None)
        } else {
            let req: Self = serde_json::from_slice(line.as_bytes())
                .map_err(MessageError::DeserializeResponse)?;
            Ok(Some(req))
        }
    }

    /// Read a response from a string slice. This is meant for the
    /// broker to use.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(line: &str) -> Result<Self, MessageError> {
        let req: Self =
            serde_json::from_slice(line.as_bytes()).map_err(MessageError::DeserializeResponse)?;
        Ok(req)
    }
}

/// All possible errors from the CI broker messages.
#[derive(Debug, thiserror::Error)]
pub enum MessageError {
    /// [`RequestBuilder`] does not have profile set.
    #[error("RequestBuilder must have profile set")]
    NoProfile,

    /// [`RequestBuilder`] does not have event set.
    #[error("RequestBuilder must have broker event set")]
    NoEvent,

    /// [`RequestBuilder`] does not have event handler set.
    #[error("RequestBuilder has no event handler set")]
    NoEventHandler,

    /// We got a CI event we don't know what to do with.
    #[error("programming error: unknown CI event {0:?}")]
    UnknownCiEvent(CiEvent),

    /// CI event was not set for [`RequestBuilder`].
    #[error("programming error: CI event was not set for request builder")]
    CiEventNotSet,

    /// Request message lacks commits to run CI on.
    #[error("unacceptable request message: lacks Git commits to run CI on")]
    NoCommits,

    /// Request message is neither a "push" nor a "patch"
    #[error("unacceptable request message: neither 'push' nor 'patch'")]
    UnknownRequest,

    /// Failed to serialize a request message as JSON. This should
    /// never happen and likely indicates a programming failure.
    #[error("failed to serialize a request into JSON to a file handle")]
    SerializeRequest(#[source] serde_json::Error),

    /// Failed to serialize a response message as JSON. This should never
    /// happen and likely indicates a programming failure.
    #[error("failed to serialize a request into JSON to a file handle")]
    SerializeResponse(#[source] serde_json::Error),

    /// Failed to write the serialized request message to an open file.
    #[error("failed to write JSON to file handle")]
    WriteRequest(#[source] std::io::Error),

    /// Failed to write the serialized response message to an open
    /// file.
    #[error("failed to write JSON to file handle")]
    WriteResponse(#[source] std::io::Error),

    /// Failed to read a line of JSON from an open file.
    #[error("failed to read line from file handle")]
    ReadLine(#[source] std::io::Error),

    /// Failed to parse JSON as a request or a response.
    #[error("failed to read a JSON request from a file handle")]
    DeserializeRequest(#[source] serde_json::Error),

    /// Failed to parse JSON as a response or a response.
    #[error("failed to read a JSON response from a file handle")]
    DeserializeResponse(#[source] serde_json::Error),

    /// Error from Radicle.
    #[error(transparent)]
    RadicleProfile(#[from] radicle::profile::Error),

    /// Error retrieving context to generate trigger.
    #[error("could not generate trigger from event")]
    Trigger,

    /// Error from Radicle storage.
    #[error(transparent)]
    StorageError(#[from] radicle::storage::Error),

    /// Error from Radicle repository.
    #[error(transparent)]
    RepositoryError(#[from] radicle::storage::RepositoryError),

    /// Error from Radicle COB.
    #[error(transparent)]
    CobStoreError(#[from] radicle::cob::store::Error),

    /// Error from `radicle-surf` crate.
    #[error(transparent)]
    RadicleSurfError(#[from] radicle_surf::Error),

    /// Trying to create a PatchAction from an invalid value.
    #[error("invalid patch action {0:?}")]
    UnknownPatchAction(String),
}

#[cfg(test)]
#[allow(clippy::unwrap_used)] // OK in tests: panic is fine
pub mod trigger_from_ci_event_tests {
    use crate::ci_event::{CiEvent, CiEventV1};
    use crate::msg::{EventType, Request, RequestBuilder};
    use radicle::git::RefString;
    use radicle::patch::{MergeTarget, Patches};
    use radicle::prelude::Did;
    use radicle::storage::ReadRepository;

    use crate::test::{MockNode, TestResult};

    #[test]
    fn trigger_push_from_branch_created() -> TestResult<()> {
        let mock_node = MockNode::new()?;
        let profile = mock_node.profile()?;

        let project = mock_node.node().project();
        let (_, repo_head) = project.repo.head()?;
        let cmt = radicle::test::fixtures::commit(
            "my test commit",
            &[repo_head.into()],
            &project.backend,
        );

        let ci_event = CiEvent::V1(CiEventV1::BranchCreated {
            from_node: *profile.id(),
            repo: project.id,
            branch: RefString::try_from(
                "refs/namespaces/$nid/refs/heads/master".replace("$nid", &profile.id().to_string()),
            )?,
            tip: cmt,
        });

        let req = RequestBuilder::default()
            .profile(&profile)
            .ci_event(&ci_event)
            .build_trigger_from_ci_event()?;
        let Request::Trigger {
            common,
            push,
            patch,
        } = req;

        assert!(patch.is_none());
        assert!(push.is_some());
        assert_eq!(common.event_type, EventType::Push);
        assert_eq!(common.repository.id, project.id);
        assert_eq!(common.repository.name, project.repo.project()?.name());

        let push = push.unwrap();
        assert_eq!(push.after, cmt);
        assert_eq!(push.before, cmt); // in this case of branch creation
        assert_eq!(
            push.branch,
            "master".replace("$nid", &profile.id().to_string())
        );
        assert_eq!(push.commits, vec![cmt]);
        assert_eq!(push.pusher.id, Did::from(profile.id()));

        Ok(())
    }

    #[test]
    fn trigger_push_from_branch_updated() -> TestResult<()> {
        let mock_node = MockNode::new()?;
        let profile = mock_node.profile()?;

        let project = mock_node.node().project();
        let (_, repo_head) = project.repo.head()?;
        let cmt = radicle::test::fixtures::commit(
            "my test commit",
            &[repo_head.into()],
            &project.backend,
        );

        let ci_event = CiEvent::V1(CiEventV1::BranchUpdated {
            from_node: *profile.id(),
            repo: project.id,
            branch: RefString::try_from(
                "refs/namespaces/$nid/refs/heads/master".replace("$nid", &profile.id().to_string()),
            )?,
            old_tip: repo_head,
            tip: cmt,
        });

        let req = RequestBuilder::default()
            .profile(&profile)
            .ci_event(&ci_event)
            .build_trigger_from_ci_event()?;
        let Request::Trigger {
            common,
            push,
            patch,
        } = req;

        assert!(patch.is_none());
        assert!(push.is_some());
        assert_eq!(common.event_type, EventType::Push);
        assert_eq!(common.repository.id, project.id);
        assert_eq!(common.repository.name, project.repo.project()?.name());

        let push = push.unwrap();
        assert_eq!(push.after, cmt);
        assert_eq!(push.before, cmt); // in this case of branch creation
        assert_eq!(
            push.branch,
            "master".replace("$nid", &profile.id().to_string())
        );
        assert_eq!(push.commits, vec![cmt]);
        assert_eq!(push.pusher.id, Did::from(profile.id()));

        Ok(())
    }

    #[test]
    fn trigger_patch_from_patch_created() -> TestResult<()> {
        let mock_node = MockNode::new()?;
        let profile = mock_node.profile()?;

        let project = mock_node.node().project();
        let (_, repo_head) = project.repo.head()?;
        let cmt = radicle::test::fixtures::commit(
            "my test commit",
            &[repo_head.into()],
            &project.backend,
        );

        let node = mock_node.node();

        let mut patches = Patches::open(&project.repo)?;
        let mut cache = radicle::cob::cache::NoCache;
        let patch_cob = patches.create(
            "my patch title",
            "my patch description",
            MergeTarget::Delegates,
            repo_head,
            cmt,
            &[],
            &mut cache,
            &node.signer,
        )?;

        let ci_event = CiEvent::V1(CiEventV1::PatchCreated {
            from_node: *profile.id(),
            repo: project.id,
            patch: *patch_cob.id(),
            new_tip: cmt,
        });

        let req = RequestBuilder::default()
            .profile(&profile)
            .ci_event(&ci_event)
            .build_trigger_from_ci_event()?;
        let Request::Trigger {
            common,
            push,
            patch,
        } = req;

        assert!(patch.is_some());
        assert!(push.is_none());
        assert_eq!(common.event_type, EventType::Patch);
        assert_eq!(common.repository.id, project.id);
        assert_eq!(common.repository.name, project.repo.project()?.name());

        let patch = patch.unwrap();
        assert_eq!(patch.action.as_str(), "created");
        assert_eq!(patch.patch.id.to_string(), patch_cob.id.to_string());
        assert_eq!(patch.patch.title, patch_cob.title());
        assert_eq!(patch.patch.state.status, patch_cob.state().to_string());
        assert_eq!(patch.patch.target, repo_head);
        assert_eq!(patch.patch.revisions.len(), 1);
        let rev = patch.patch.revisions.first().unwrap();
        assert_eq!(rev.id.to_string(), patch_cob.id.to_string());
        assert_eq!(rev.base, repo_head);
        assert_eq!(rev.oid, cmt);
        assert_eq!(rev.author.id, Did::from(profile.id()));
        assert_eq!(rev.description, patch_cob.description());
        assert_eq!(rev.timestamp, patch_cob.timestamp().as_secs());
        assert_eq!(patch.patch.after, cmt);
        assert_eq!(patch.patch.before, repo_head);
        assert_eq!(patch.patch.commits, vec![cmt]);

        Ok(())
    }

    #[test]
    fn trigger_patch_from_patch_updated() -> TestResult<()> {
        let mock_node = MockNode::new()?;
        let profile = mock_node.profile()?;

        let project = mock_node.node().project();
        let (_, repo_head) = project.repo.head()?;
        let cmt = radicle::test::fixtures::commit(
            "my test commit",
            &[repo_head.into()],
            &project.backend,
        );

        let node = mock_node.node();

        let mut patches = Patches::open(&project.repo)?;
        let mut cache = radicle::cob::cache::NoCache;
        let patch_cob = patches.create(
            "my patch title",
            "my patch description",
            MergeTarget::Delegates,
            repo_head,
            cmt,
            &[],
            &mut cache,
            &node.signer,
        )?;

        let ci_event = CiEvent::V1(CiEventV1::PatchUpdated {
            from_node: *profile.id(),
            repo: project.id,
            patch: *patch_cob.id(),
            new_tip: cmt,
        });

        let req = RequestBuilder::default()
            .profile(&profile)
            .ci_event(&ci_event)
            .build_trigger_from_ci_event()?;
        let Request::Trigger {
            common,
            push,
            patch,
        } = req;

        assert!(patch.is_some());
        assert!(push.is_none());
        assert_eq!(common.event_type, EventType::Patch);
        assert_eq!(common.repository.id, project.id);
        assert_eq!(common.repository.name, project.repo.project()?.name());

        let patch = patch.unwrap();
        assert_eq!(patch.action.as_str(), "updated");
        assert_eq!(patch.patch.id.to_string(), patch_cob.id.to_string());
        assert_eq!(patch.patch.title, patch_cob.title());
        assert_eq!(patch.patch.state.status, patch_cob.state().to_string());
        assert_eq!(patch.patch.target, repo_head);
        assert_eq!(patch.patch.revisions.len(), 1);
        let rev = patch.patch.revisions.first().unwrap();
        assert_eq!(rev.id.to_string(), patch_cob.id.to_string());
        assert_eq!(rev.base, repo_head);
        assert_eq!(rev.oid, cmt);
        assert_eq!(rev.author.id, Did::from(profile.id()));
        assert_eq!(rev.description, patch_cob.description());
        assert_eq!(rev.timestamp, patch_cob.timestamp().as_secs());
        assert_eq!(patch.patch.after, cmt);
        assert_eq!(patch.patch.before, repo_head);
        assert_eq!(patch.patch.commits, vec![cmt]);

        Ok(())
    }
}

// Parse a Git ref to get the branch it refers to. If it doesn't
// return to a branch, return the empty string.
fn push_branch(name: &str) -> String {
    let mut parts = name.split("/refs/heads/");
    if let Some(suffix) = parts.nth(1) {
        if parts.next().is_none() {
            return suffix.to_string();
        }
    }
    "".to_string()
}

#[cfg(test)]
mod test_push_branch {
    use super::push_branch;

    #[test]
    fn get_push_branch() {
        assert_eq!(
            push_branch(
                "refs/namespaces/z6MkuhvCnrcow7vzkyQzkuFixzpTa42iC2Cfa4DA8HRLCmys/refs/heads/branch_name"
            ),
            "branch_name".to_string()
        );
    }

    #[test]
    fn get_no_push_branch() {
        assert_eq!(
            push_branch(
                "refs/namespaces/z6MkuhvCnrcow7vzkyQzkuFixzpTa42iC2Cfa4DA8HRLCmys/refs/rad/sigrefs"
            ),
            "".to_string()
        );
    }
}
