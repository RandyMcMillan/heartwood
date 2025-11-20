use std::path::{Path, PathBuf};

use regex::Regex;
use serde::{Deserialize, Serialize};

use radicle_git_ext::Oid;

use radicle::{
    cob::patch::PatchId,
    git::RefString,
    node::{Event, NodeId},
    prelude::RepoId,
    storage::RefUpdate,
};

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum CiEvent {
    V1(CiEventV1),
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum CiEventV1 {
    Shutdown,
    BranchCreated {
        from_node: NodeId,
        repo: RepoId,
        branch: RefString,
        tip: Oid,
    },
    BranchUpdated {
        from_node: NodeId,
        repo: RepoId,
        branch: RefString,
        tip: Oid,
        old_tip: Oid,
    },
    BranchDeleted {
        from_node: NodeId,
        repo: RepoId,
        branch: RefString,
        tip: Oid,
    },
    PatchCreated {
        from_node: NodeId,
        repo: RepoId,
        patch: PatchId,
        new_tip: Oid,
    },
    PatchUpdated {
        from_node: NodeId,
        repo: RepoId,
        patch: PatchId,
        new_tip: Oid,
    },
}

impl CiEvent {
    pub fn branch_created(
        node: NodeId,
        repo: RepoId,
        branch: &str,
        tip: Oid,
    ) -> Result<Self, CiEventError> {
        Ok(Self::V1(CiEventV1::BranchCreated {
            from_node: node,
            repo,
            branch: RefString::try_from(branch)
                .map_err(|e| CiEventError::RefString(branch.into(), e))?,
            tip,
        }))
    }

    pub fn branch_updated(
        node: NodeId,
        repo: RepoId,
        branch: &str,
        tip: Oid,
        old_tip: Oid,
    ) -> Result<Self, CiEventError> {
        let branch =
            namespaced_branch(branch).map_err(|_| CiEventError::without_namespace2(branch))?;
        Ok(Self::V1(CiEventV1::BranchUpdated {
            from_node: node,
            repo,
            branch: RefString::try_from(branch.clone())
                .map_err(|e| CiEventError::RefString(branch.clone(), e))?,
            tip,
            old_tip,
        }))
    }

    pub fn from_node_event(event: &Event) -> Result<Vec<Self>, CiEventError> {
        fn ref_string(s: String) -> Result<RefString, CiEventError> {
            RefString::try_from(s.clone()).map_err(|e| CiEventError::ref_string(s, e))
        }

        fn branch(ref_name: &str, update: &RefUpdate) -> Result<RefString, CiEventError> {
            ref_string(
                namespaced_branch(ref_name)
                    .map_err(|_| CiEventError::without_namespace(ref_name, update.clone()))?,
            )
        }

        match event {
            Event::RefsFetched {
                remote,
                rid,
                updated,
            } => {
                let mut events = vec![];
                for update in updated {
                    let e = match update {
                        RefUpdate::Created { name, oid } => {
                            eprintln!("created: {name:?}");
                            if let Ok(patch_id) = patch_id(name) {
                                CiEventV1::PatchCreated {
                                    from_node: *remote,
                                    repo: *rid,
                                    patch: patch_id,
                                    new_tip: *oid,
                                }
                            } else if let Ok(branch) = namespaced_branch(name) {
                                CiEventV1::BranchCreated {
                                    from_node: *remote,
                                    repo: *rid,
                                    branch: ref_string(branch)?,
                                    tip: *oid,
                                }
                            } else {
                                eprintln!("don't know what it is, ignoring");
                                continue;
                            }
                        }
                        RefUpdate::Updated { name, old, new } => {
                            eprintln!("updated: {name:?}");
                            if let Ok(patch_id) = patch_id(name) {
                                eprintln!("it's a patch");
                                CiEventV1::PatchUpdated {
                                    from_node: *remote,
                                    repo: *rid,
                                    patch: patch_id,
                                    new_tip: *new,
                                }
                            } else if let Ok(branch) = namespaced_branch(name) {
                                eprintln!("it's a branch update");
                                CiEventV1::BranchUpdated {
                                    from_node: *remote,
                                    repo: *rid,
                                    branch: ref_string(branch)?,
                                    tip: *new,
                                    old_tip: *old,
                                }
                            } else {
                                eprintln!("don't know what it is, ignoring");
                                continue;
                            }
                        }
                        RefUpdate::Deleted { name, oid } => CiEventV1::BranchDeleted {
                            from_node: *remote,
                            repo: *rid,
                            branch: branch(name, update)?,
                            tip: *oid,
                        },
                        RefUpdate::Skipped { .. } => continue,
                    };
                    events.push(CiEvent::V1(e));
                }
                Ok(events)
            }
            Event::RefsSynced { .. }
            | Event::RefsAnnounced { .. }
            | Event::NodeAnnounced { .. }
            | Event::SeedDiscovered { .. }
            | Event::SeedDropped { .. }
            | Event::PeerConnected { .. }
            | Event::PeerDisconnected { .. }
            | Event::LocalRefsAnnounced { .. }
            | Event::UploadPack { .. }
            | Event::InventoryAnnounced { .. } => Ok(vec![]),
        }
    }
}

pub struct CiEvents {
    events: Vec<CiEvent>,
}

impl CiEvents {
    pub fn from_file(filename: &Path) -> Result<Self, CiEventError> {
        let events = std::fs::read(filename).map_err(|e| CiEventError::read_file(filename, e))?;
        let events = String::from_utf8(events).map_err(|e| CiEventError::not_utf8(filename, e))?;
        let events: Result<Vec<CiEvent>, _> = events.lines().map(serde_json::from_str).collect();
        let events = events.map_err(|e| CiEventError::not_json(filename, e))?;

        Ok(Self { events })
    }

    pub fn iter(&self) -> impl Iterator<Item = &CiEvent> {
        self.events.iter()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CiEventError {
    #[error("updated ref name has no name space: {0:?}): from {1:#?}")]
    WithoutNamespace(String, RefUpdate),

    #[error("updated ref name has no name space: {0:?})")]
    WithoutNamespace2(String),

    #[error("failed to create a RefString from {0:?}")]
    RefString(String, radicle::git::fmt::Error),

    #[error("failed to read broker events file {0}")]
    ReadFile(PathBuf, #[source] std::io::Error),

    #[error("broker events file is not UTF8: {0}")]
    NotUtf8(PathBuf, #[source] std::string::FromUtf8Error),

    #[error("broker events file is not valid JSON: {0}")]
    NotJson(PathBuf, #[source] serde_json::Error),
}

impl CiEventError {
    fn without_namespace(refname: &str, update: RefUpdate) -> Self {
        Self::WithoutNamespace(refname.into(), update)
    }

    fn without_namespace2(refname: &str) -> Self {
        Self::WithoutNamespace2(refname.into())
    }

    fn ref_string(name: String, err: radicle::git::fmt::Error) -> Self {
        Self::RefString(name, err)
    }

    fn read_file(filename: &Path, err: std::io::Error) -> Self {
        Self::ReadFile(filename.into(), err)
    }

    fn not_utf8(filename: &Path, err: std::string::FromUtf8Error) -> Self {
        Self::NotUtf8(filename.into(), err)
    }

    fn not_json(filename: &Path, err: serde_json::Error) -> Self {
        Self::NotJson(filename.into(), err)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod test {
    use super::*;
    use radicle::{prelude::NodeId, storage::RefUpdate};
    use std::str::FromStr;

    const MAIN_BRANCH_REF_NAME: &str = "refs/namespaces/NID/refs/heads/main";
    const MAIN_BRANCH_NAME: &str = "main";

    const PATCH_REF_NAME: &str =
        "refs/namespaces/NID/refs/heads/patches/f9fa90725474de9002be503ae3cda4670c9a174";
    const PATCH_ID: &str = "f9fa90725474de9002be503ae3cda4670c9a174";

    fn nid() -> NodeId {
        const NID: &str = "z6MkgEMYod7Hxfy9qCvDv5hYHkZ4ciWmLFgfvm3Wn1b2w2FV";
        NodeId::from_str(NID).unwrap()
    }

    fn rid() -> RepoId {
        const RID: &str = "rad:z3gqcJUoA1n9HaHKufZs5FCSGazv5";
        RepoId::from_urn(RID).unwrap()
    }

    fn oid_from(oid: &str) -> Oid {
        Oid::try_from(oid).unwrap()
    }

    fn oid() -> Oid {
        const OID: &str = "ff3099ba5de28d954c41d0b5a84316f943794ea4";
        oid_from(OID)
    }

    fn refstring(s: &str) -> RefString {
        RefString::try_from(s).unwrap()
    }

    #[test]
    fn nothing_updated() {
        let event = Event::RefsFetched {
            remote: nid(),
            rid: rid(),
            updated: vec![],
        };
        let result = CiEvent::from_node_event(&event);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![]);
    }

    #[test]
    fn skipped() {
        let event = Event::RefsFetched {
            remote: nid(),
            rid: rid(),
            updated: vec![RefUpdate::Skipped {
                name: refstring(MAIN_BRANCH_REF_NAME),
                oid: oid(),
            }],
        };

        let result = CiEvent::from_node_event(&event);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![]);
    }

    #[test]
    fn branch_created() {
        let rid = rid();
        let main = refstring(MAIN_BRANCH_NAME);
        let oid = oid();
        let event = Event::RefsFetched {
            remote: nid(),
            rid,
            updated: vec![RefUpdate::Created {
                name: refstring(MAIN_BRANCH_REF_NAME),
                oid,
            }],
        };
        let x = CiEvent::from_node_event(&event);
        eprintln!("result: {x:#?}");
        match x {
            Err(_) => panic!("should succeed"),
            Ok(events) if !events.is_empty() => {
                for e in events {
                    match e {
                        CiEvent::V1(CiEventV1::BranchCreated {
                            from_node: _,
                            repo,
                            branch,
                            tip,
                        }) if repo == rid && branch == main && tip == oid => {}
                        _ => panic!("should not succeed that way"),
                    }
                }
            }
            Ok(_) => panic!("empty list of events should not happen"),
        }
    }

    #[test]
    fn branch_updated() {
        let rid = rid();
        let main = refstring(MAIN_BRANCH_NAME);
        let oid = oid();
        let event = Event::RefsFetched {
            remote: nid(),
            rid,
            updated: vec![RefUpdate::Updated {
                name: refstring(MAIN_BRANCH_REF_NAME),
                old: oid,
                new: oid,
            }],
        };
        let x = CiEvent::from_node_event(&event);
        eprintln!("result: {x:#?}");
        match x {
            Err(_) => panic!("should succeed"),
            Ok(events) if !events.is_empty() => {
                for e in events {
                    match e {
                        CiEvent::V1(CiEventV1::BranchUpdated {
                            from_node: _,
                            repo,
                            branch,
                            tip,
                            old_tip,
                        }) if repo == rid && branch == main && tip == oid && old_tip == oid => {}
                        _ => panic!("should not succeed that way"),
                    }
                }
            }
            Ok(_) => panic!("empty list of events should not happen"),
        }
    }

    #[test]
    fn branch_deleted() {
        let rid = rid();
        let main = refstring(MAIN_BRANCH_NAME);
        let oid = oid();
        let event = Event::RefsFetched {
            remote: nid(),
            rid,
            updated: vec![RefUpdate::Deleted {
                name: refstring(MAIN_BRANCH_REF_NAME),
                oid,
            }],
        };
        let x = CiEvent::from_node_event(&event);
        eprintln!("result: {x:#?}");
        match x {
            Err(_) => panic!("should succeed"),
            Ok(events) if !events.is_empty() => {
                for e in events {
                    match e {
                        CiEvent::V1(CiEventV1::BranchDeleted {
                            repo, branch, tip, ..
                        }) if repo == rid && branch == main && tip == oid => {}
                        _ => panic!("should not succeed that way"),
                    }
                }
            }
            Ok(_) => panic!("empty list of events should not happen"),
        }
    }

    #[test]
    fn patch_created() {
        let rid = rid();
        let patch_id = oid_from(PATCH_ID).into();
        let oid = oid();
        let event = Event::RefsFetched {
            remote: nid(),
            rid,
            updated: vec![RefUpdate::Created {
                name: refstring(PATCH_REF_NAME),
                oid,
            }],
        };
        let x = CiEvent::from_node_event(&event);
        eprintln!("result: {x:#?}");
        match x {
            Err(_) => panic!("should succeed"),
            Ok(events) if !events.is_empty() => {
                for e in events {
                    match e {
                        CiEvent::V1(CiEventV1::PatchCreated {
                            from_node: _,
                            repo,
                            patch,
                            new_tip,
                        }) if repo == rid && patch == patch_id && new_tip == oid => {}
                        _ => panic!("should not succeed that way"),
                    }
                }
            }
            Ok(_) => panic!("empty list of events should not happen"),
        }
    }

    #[test]
    fn patch_updated() {
        let rid = rid();
        let patch_id = oid_from(PATCH_ID).into();
        let oid = oid();
        let event = Event::RefsFetched {
            remote: nid(),
            rid,
            updated: vec![RefUpdate::Updated {
                name: refstring(PATCH_REF_NAME),
                old: oid,
                new: oid,
            }],
        };
        let x = CiEvent::from_node_event(&event);
        eprintln!("result: {x:#?}");
        match x {
            Err(_) => panic!("should succeed"),
            Ok(events) if !events.is_empty() => {
                for e in events {
                    match e {
                        CiEvent::V1(CiEventV1::PatchUpdated {
                            from_node: _,
                            repo,
                            patch,
                            new_tip,
                        }) if repo == rid && patch == patch_id && new_tip == oid => {}
                        _ => panic!("should not succeed that way"),
                    }
                }
            }
            Ok(_) => panic!("empty list of events should not happen"),
        }
    }
}

fn namespaced_branch(refname: &str) -> Result<String, ParseError> {
    const PAT_BRANCH: &str = r"^refs/namespaces/[^/]+/refs/heads/(.+)$";
    let push_re = Regex::new(PAT_BRANCH).map_err(|e| ParseError::Regex(PAT_BRANCH, e))?;
    if let Some(push_captures) = push_re.captures(refname) {
        if let Some(branch) = push_captures.get(1) {
            return Ok(branch.as_str().to_string());
        }
    }
    Err(ParseError::NotBranch(refname.into()))
}

fn patch_id(refname: &str) -> Result<PatchId, ParseError> {
    eprintln!("refname: {refname:?}");
    const PAT_PATCH: &str = r"^refs/namespaces/[^/]+/refs/heads/patches/([^/]+)$";
    let patch_re = Regex::new(PAT_PATCH).map_err(|e| ParseError::regex(PAT_PATCH, e))?;
    if let Some(patch_captures) = patch_re.captures(refname) {
        eprintln!("captures: {patch_captures:?}");
        if let Some(patch_id) = patch_captures.get(1) {
            eprintln!("patch_id: {patch_id:?}");
            let oid = Oid::try_from(patch_id.as_str())
                .map_err(|e| ParseError::oid(patch_id.as_str(), e))?;
            eprintln!("oid: {oid:?}");
            return Ok(oid.into());
        }
    }

    Err(ParseError::not_patch(refname))
}

#[derive(Debug, thiserror::Error)]
enum ParseError {
    #[error("programming error: unacceptable regular expression {0:?}")]
    Regex(&'static str, regex::Error),

    #[error("Git ref name without name space: {0:?}")]
    NotBranch(String),

    #[error("unacceptable Git ref for patch: {0:?}")]
    NotPatch(String),

    #[error("Git ref name includes unacceptable Git object id: {0:?}")]
    Oid(String, radicle::git::raw::Error),
}

impl ParseError {
    fn regex(pattern: &'static str, err: regex::Error) -> Self {
        Self::Regex(pattern, err)
    }

    fn not_patch(refname: &str) -> Self {
        Self::NotPatch(refname.into())
    }

    fn oid(refname: &str, err: radicle::git::raw::Error) -> Self {
        Self::Oid(refname.into(), err)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod test_namespaced_branch {
    use super::{namespaced_branch, ParseError};

    #[test]
    fn empty() {
        assert!(matches!(
            namespaced_branch(""),
            Err(ParseError::NotBranch(_))
        ));
    }

    #[test]
    fn lacks_namespace() {
        assert!(matches!(
            namespaced_branch(""),
            Err(ParseError::NotBranch(_))
        ));
    }

    #[test]
    fn has_namespace() {
        let x = namespaced_branch("refs/namespaces/DID/refs/heads/main");
        assert!(x.is_ok());
        assert_eq!(x.unwrap(), "main");
    }

    #[test]
    fn has_namespace_with_path() {
        let x = namespaced_branch("refs/namespaces/DID/refs/heads/liw/debug/this/path");
        assert!(x.is_ok());
        assert_eq!(x.unwrap(), "liw/debug/this/path");
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod test_patch_id {
    use super::{patch_id, Oid, ParseError};

    #[test]
    fn empty() {
        assert!(matches!(patch_id(""), Err(ParseError::NotPatch(_))));
    }

    #[test]
    fn lacks_namespace() {
        assert!(matches!(patch_id(""), Err(ParseError::NotPatch(_))));
    }

    #[test]
    fn has_namespace() {
        let x = patch_id(
            "refs/namespaces/DID/refs/heads/patches/f9fa90725474de9002be503ae3cda4670c9a1741",
        );
        assert!(x.is_ok());
        assert_eq!(
            x.unwrap(),
            Oid::try_from("f9fa90725474de9002be503ae3cda4670c9a1741")
                .unwrap()
                .into()
        );
    }

    #[test]
    fn has_namespace_with_path() {
        assert!(matches!(
            patch_id("refs/namespaces/DID/refs/heads/patches/coffee/beef"),
            Err(ParseError::NotPatch(_))
        ));
    }
}
