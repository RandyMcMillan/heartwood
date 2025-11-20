use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use radicle::{cob::patch::PatchId, git::RefString, node::NodeId, prelude::RepoId};
use radicle_git_ext::Oid;

use crate::{
    ci_event::{CiEvent, CiEventV1},
    logger,
};

/// A Boolean expression for filtering broker events.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum EventFilter {
    /// Event for a specific repository.
    Repository(RepoId),

    /// Event is for a specific branch.
    Branch(RefString),

    /// Branch was created.
    BranchCreated,

    /// Branch was updated.
    BranchUpdated,

    /// Branch was deleted.
    BranchDeleted,

    /// Event is for a specific patch.
    Patch(Oid),

    /// Patch was created.
    PatchCreated,

    /// Patch was updated,
    PatchUpdated,

    /// Change originated from specific node.
    Node(NodeId),

    /// Allow any event.
    Allow,

    /// Don't allow any event.
    Deny,

    /// Allow the opposite of the contained filter.
    Not(Box<Self>),

    /// Allow if all contained filters allow.
    And(Vec<Box<Self>>),

    /// Allow if any contained filter allow.
    Or(Vec<Box<Self>>),
}

impl EventFilter {
    pub fn allows(&self, event: &CiEvent) -> bool {
        match self {
            Self::Allow => return true,
            Self::Deny => return false,
            Self::Not(expr) => return !expr.allows(event),
            Self::And(exprs) => return exprs.iter().all(|e| e.allows(event)),
            Self::Or(exprs) => return exprs.iter().any(|e| e.allows(event)),
            _ => (),
        }

        let decision = match event {
            CiEvent::V1(CiEventV1::Shutdown) => true,
            CiEvent::V1(CiEventV1::BranchCreated {
                from_node,
                repo,
                branch,
                ..
            }) => match self {
                Self::Node(wantedc) => from_node == wantedc,
                Self::Repository(wanted) => repo == wanted,
                Self::Branch(wanted) => branch == wanted,
                Self::BranchCreated => true,
                _ => false,
            },
            CiEvent::V1(CiEventV1::BranchUpdated {
                from_node,
                repo,
                branch,
                ..
            }) => match self {
                Self::Node(wanted) => from_node == wanted,
                Self::Repository(wanted) => repo == wanted,
                Self::Branch(wanted) => branch == wanted,
                Self::BranchUpdated => true,
                _ => false,
            },
            CiEvent::V1(CiEventV1::BranchDeleted {
                from_node,
                repo,
                branch,
                ..
            }) => match self {
                Self::Node(wanted) => from_node == wanted,
                Self::Repository(wanted) => repo == wanted,
                Self::Branch(wanted) => branch == wanted,
                Self::BranchDeleted => true,
                _ => false,
            },
            CiEvent::V1(CiEventV1::PatchCreated {
                from_node,
                repo,
                patch,
                ..
            }) => match self {
                Self::Node(wanted) => from_node == wanted,
                Self::Repository(wanted) => repo == wanted,
                Self::Patch(wanted) => *patch == PatchId::from(wanted),
                Self::PatchCreated => true,
                _ => false,
            },
            CiEvent::V1(CiEventV1::PatchUpdated {
                from_node,
                repo,
                patch,
                ..
            }) => match self {
                Self::Node(wanted) => from_node == wanted,
                Self::Repository(wanted) => repo == wanted,
                Self::Patch(wanted) => *patch == PatchId::from(wanted),
                Self::PatchUpdated => true,
                _ => false,
            },
        };

        logger::debug2(format!(
            "EventFilter::allows: decision={decision} event={event:?}"
        ));

        decision
    }

    pub fn from_file(filename: &Path) -> Result<Vec<Self>, FilterError> {
        Filters::from_file(filename)
    }
}

#[derive(Deserialize)]
struct Filters {
    filters: Vec<EventFilter>,
}

impl Filters {
    fn from_file(filename: &Path) -> Result<Vec<EventFilter>, FilterError> {
        let data =
            std::fs::read(filename).map_err(|e| FilterError::ReadFile(filename.into(), e))?;
        let filters: Self =
            serde_yml::from_slice(&data).map_err(|e| FilterError::ParseYaml(filename.into(), e))?;
        Ok(filters.filters)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod test {
    use radicle::prelude::{Did, RepoId};

    use super::*;

    fn did() -> Did {
        Did::decode("did:key:z6MkgEMYod7Hxfy9qCvDv5hYHkZ4ciWmLFgfvm3Wn1b2w2FV").unwrap()
    }

    fn other_did() -> Did {
        Did::decode("did:key:z6MkfXa53s1ZSFy8rktvyXt5ADCojnxvjAoQpzajaXyLqG5n").unwrap()
    }

    fn rid() -> RepoId {
        const RID: &str = "rad:z3gqcJUoA1n9HaHKufZs5FCSGazv5";
        RepoId::from_urn(RID).unwrap()
    }

    fn other_rid() -> RepoId {
        const RID: &str = "rad:zwTxygwuz5LDGBq255RA2CbNGrz8";
        RepoId::from_urn(RID).unwrap()
    }

    fn oid_from(oid: &str) -> Oid {
        Oid::try_from(oid).unwrap()
    }

    fn oid() -> Oid {
        const OID: &str = "ff3099ba5de28d954c41d0b5a84316f943794ea4";
        oid_from(OID)
    }

    fn other_oid() -> Oid {
        const OID: &str = "bde68ac76ce093bcc583aa612f45e13fee2353a0";
        oid_from(OID)
    }

    fn patch_id() -> PatchId {
        PatchId::from(oid())
    }

    fn other_patch_id() -> PatchId {
        PatchId::from(other_oid())
    }

    fn refstring(s: &str) -> RefString {
        RefString::try_from(s).unwrap()
    }

    fn shutdown() -> CiEvent {
        CiEvent::V1(CiEventV1::Shutdown)
    }

    fn all_events(
        did: Did,
        repo: RepoId,
        branch: RefString,
        patch: PatchId,
        tip: Oid,
        old_tip: Oid,
    ) -> Vec<CiEvent> {
        vec![
            CiEvent::V1(CiEventV1::BranchCreated {
                from_node: did.into(),
                repo,
                branch: branch.clone(),
                tip,
            }),
            CiEvent::V1(CiEventV1::BranchUpdated {
                from_node: did.into(),
                repo,
                branch: branch.clone(),
                tip,
                old_tip,
            }),
            CiEvent::V1(CiEventV1::BranchDeleted {
                from_node: did.into(),
                repo,
                branch,
                tip,
            }),
            CiEvent::V1(CiEventV1::PatchCreated {
                from_node: did.into(),
                repo,
                patch,
                new_tip: tip,
            }),
            CiEvent::V1(CiEventV1::PatchUpdated {
                from_node: did.into(),
                repo,
                patch,
                new_tip: tip,
            }),
        ]
    }

    // Verify that shutdown is allowed, even when filtering for
    // something else.
    #[test]
    fn allows_shutdown() {
        let filter = EventFilter::Repository(rid());
        assert!(filter.allows(&shutdown()))
    }

    #[test]
    fn allows_all_for_default_repository() {
        let filter = EventFilter::Repository(rid());
        let events = all_events(did(), rid(), refstring("main"), patch_id(), oid(), oid());
        assert!(events.iter().all(|e| filter.allows(e)));
    }

    #[test]
    fn doesnt_allow_any_for_other_repository() {
        let filter = EventFilter::Repository(rid());
        let events = all_events(
            did(),
            other_rid(),
            refstring("main"),
            patch_id(),
            oid(),
            oid(),
        );
        eprintln!("filter: {filter:#?}");
        for e in events.iter() {
            eprintln!("{:#?} → {}", e, filter.allows(e));
            assert!(!filter.allows(e));
        }
    }

    #[test]
    fn allows_all_for_main_branch() {
        let filter = EventFilter::Branch(refstring("main"));
        let events = all_events(did(), rid(), refstring("main"), patch_id(), oid(), oid());
        eprintln!("filter: {filter:#?}");
        for e in events.iter().filter(|e| {
            matches!(
                e,
                CiEvent::V1(CiEventV1::BranchCreated { .. })
                    | CiEvent::V1(CiEventV1::BranchUpdated { .. })
                    | CiEvent::V1(CiEventV1::BranchDeleted { .. })
            )
        }) {
            eprintln!("{:#?} → {}", e, filter.allows(e));
            assert!(filter.allows(e));
        }
    }

    #[test]
    fn doesnt_allow_any_for_other_branch() {
        let filter = EventFilter::Branch(refstring("main"));
        let events = all_events(
            did(),
            other_rid(),
            refstring("other"),
            patch_id(),
            oid(),
            oid(),
        );
        eprintln!("filter: {filter:#?}");
        for e in events.iter() {
            eprintln!("{:#?} → {}", e, filter.allows(e));
            assert!(!filter.allows(e));
        }
    }

    #[test]
    fn allows_branch_creation() {
        let filter = EventFilter::BranchCreated;

        eprintln!("filter: {filter:#?}");
        for e in all_events(did(), rid(), refstring("main"), patch_id(), oid(), oid())
            .iter()
            .filter(|e| matches!(e, CiEvent::V1(CiEventV1::BranchCreated { .. })))
        {
            eprintln!("{:#?} → {}", e, filter.allows(e));
            assert!(filter.allows(e));
        }
    }

    #[test]
    fn only_allows_branch_creation() {
        let filter = EventFilter::BranchCreated;

        eprintln!("filter: {filter:#?}");
        for e in all_events(did(), rid(), refstring("main"), patch_id(), oid(), oid())
            .iter()
            .filter(|e| !matches!(e, CiEvent::V1(CiEventV1::BranchCreated { .. })))
        {
            eprintln!("{:#?} → {}", e, filter.allows(e));
            assert!(!filter.allows(e));
        }
    }

    #[test]
    fn allows_branch_update() {
        let filter = EventFilter::BranchUpdated;

        eprintln!("filter: {filter:#?}");
        for e in all_events(did(), rid(), refstring("main"), patch_id(), oid(), oid())
            .iter()
            .filter(|e| matches!(e, CiEvent::V1(CiEventV1::BranchUpdated { .. })))
        {
            eprintln!("{:#?} → {}", e, filter.allows(e));
            assert!(filter.allows(e));
        }
    }

    #[test]
    fn only_allows_branch_update() {
        let filter = EventFilter::BranchUpdated;

        eprintln!("filter: {filter:#?}");
        for e in all_events(did(), rid(), refstring("main"), patch_id(), oid(), oid())
            .iter()
            .filter(|e| !matches!(e, CiEvent::V1(CiEventV1::BranchUpdated { .. })))
        {
            eprintln!("{:#?} → {}", e, filter.allows(e));
            assert!(!filter.allows(e));
        }
    }

    #[test]
    fn allows_branch_deletion() {
        let filter = EventFilter::BranchDeleted;

        eprintln!("filter: {filter:#?}");
        for e in all_events(did(), rid(), refstring("main"), patch_id(), oid(), oid())
            .iter()
            .filter(|e| matches!(e, CiEvent::V1(CiEventV1::BranchDeleted { .. })))
        {
            eprintln!("{:#?} → {}", e, filter.allows(e));
            assert!(filter.allows(e));
        }
    }

    #[test]
    fn only_allows_branch_deletion() {
        let filter = EventFilter::BranchDeleted;

        eprintln!("filter: {filter:#?}");
        for e in all_events(did(), rid(), refstring("main"), patch_id(), oid(), oid())
            .iter()
            .filter(|e| !matches!(e, CiEvent::V1(CiEventV1::BranchDeleted { .. })))
        {
            eprintln!("{:#?} → {}", e, filter.allows(e));
            assert!(!filter.allows(e));
        }
    }

    #[test]
    fn allows_specific_patch() {
        let filter = EventFilter::Patch(oid());
        let events = all_events(did(), rid(), refstring("main"), patch_id(), oid(), oid());
        eprintln!("filter: {filter:#?}");
        for e in events.iter().filter(|e| {
            matches!(
                e,
                CiEvent::V1(CiEventV1::PatchCreated { .. })
                    | CiEvent::V1(CiEventV1::PatchUpdated { .. })
            )
        }) {
            eprintln!("{:#?} → {}", e, filter.allows(e));
            assert!(filter.allows(e));
        }
    }

    #[test]
    fn doesnt_allows_other_patch() {
        let filter = EventFilter::Patch(oid());
        let events = all_events(
            did(),
            rid(),
            refstring("main"),
            other_patch_id(),
            oid(),
            oid(),
        );
        eprintln!("filter: {filter:#?}");
        for e in events.iter().filter(|e| {
            matches!(
                e,
                CiEvent::V1(CiEventV1::PatchCreated { .. })
                    | CiEvent::V1(CiEventV1::PatchUpdated { .. })
            )
        }) {
            eprintln!("{:#?} → {}", e, filter.allows(e));
            assert!(!filter.allows(e));
        }
    }

    #[test]
    fn allows_patch_creation() {
        let filter = EventFilter::PatchCreated;

        eprintln!("filter: {filter:#?}");
        for e in all_events(did(), rid(), refstring("main"), patch_id(), oid(), oid())
            .iter()
            .filter(|e| matches!(e, CiEvent::V1(CiEventV1::PatchCreated { .. })))
        {
            eprintln!("{:#?} → {}", e, filter.allows(e));
            assert!(filter.allows(e));
        }
    }

    #[test]
    fn only_allows_patch_creation() {
        let filter = EventFilter::PatchCreated;

        eprintln!("filter: {filter:#?}");
        for e in all_events(did(), rid(), refstring("main"), patch_id(), oid(), oid())
            .iter()
            .filter(|e| !matches!(e, CiEvent::V1(CiEventV1::PatchCreated { .. })))
        {
            eprintln!("{:#?} → {}", e, filter.allows(e));
            assert!(!filter.allows(e));
        }
    }

    #[test]
    fn allows_patch_update() {
        let filter = EventFilter::PatchUpdated;

        eprintln!("filter: {filter:#?}");
        for e in all_events(did(), rid(), refstring("main"), patch_id(), oid(), oid())
            .iter()
            .filter(|e| matches!(e, CiEvent::V1(CiEventV1::PatchUpdated { .. })))
        {
            eprintln!("{:#?} → {}", e, filter.allows(e));
            assert!(filter.allows(e));
        }
    }

    #[test]
    fn only_allows_patch_update() {
        let filter = EventFilter::PatchUpdated;

        eprintln!("filter: {filter:#?}");
        for e in all_events(did(), rid(), refstring("main"), patch_id(), oid(), oid())
            .iter()
            .filter(|e| !matches!(e, CiEvent::V1(CiEventV1::PatchUpdated { .. })))
        {
            eprintln!("{:#?} → {}", e, filter.allows(e));
            assert!(!filter.allows(e));
        }
    }

    #[test]
    fn allows_all_for_right_node() {
        let filter = EventFilter::Node(*did());
        let events = all_events(did(), rid(), refstring("main"), patch_id(), oid(), oid());
        assert!(events.iter().all(|e| filter.allows(e)));
    }

    #[test]
    fn allows_none_for_wrong_node() {
        let filter = EventFilter::Node(*other_did());
        let events = all_events(did(), rid(), refstring("main"), patch_id(), oid(), oid());
        assert!(!events.iter().any(|e| filter.allows(e)));
    }

    #[test]
    fn allows_any_event() {
        let filter = EventFilter::Allow;

        eprintln!("filter: {filter:#?}");
        for e in all_events(did(), rid(), refstring("main"), patch_id(), oid(), oid()).iter() {
            eprintln!("{:#?} → {}", e, filter.allows(e));
            assert!(filter.allows(e));
        }
    }

    #[test]
    fn allows_no_event() {
        let filter = EventFilter::Deny;

        eprintln!("filter: {filter:#?}");
        for e in all_events(did(), rid(), refstring("main"), patch_id(), oid(), oid()).iter() {
            eprintln!("{:#?} → {}", e, filter.allows(e));
            assert!(!filter.allows(e));
        }
    }

    #[test]
    fn allows_opposite() {
        let filter = EventFilter::Not(Box::new(EventFilter::Deny));

        eprintln!("filter: {filter:#?}");
        for e in all_events(did(), rid(), refstring("main"), patch_id(), oid(), oid()).iter() {
            eprintln!("{:#?} → {}", e, filter.allows(e));
            assert!(filter.allows(e));
        }
    }

    #[test]
    fn allows_if_all_allow() {
        let filter = EventFilter::And(vec![
            Box::new(EventFilter::Allow),
            Box::new(EventFilter::Allow),
        ]);

        eprintln!("filter: {filter:#?}");
        for e in all_events(did(), rid(), refstring("main"), patch_id(), oid(), oid()).iter() {
            eprintln!("{:#?} → {}", e, filter.allows(e));
            assert!(filter.allows(e));
        }
    }

    #[test]
    fn allows_if_any_allows() {
        let filter = EventFilter::Or(vec![
            Box::new(EventFilter::Deny),
            Box::new(EventFilter::Allow),
        ]);

        eprintln!("filter: {filter:#?}");
        for e in all_events(did(), rid(), refstring("main"), patch_id(), oid(), oid()).iter() {
            eprintln!("{:#?} → {}", e, filter.allows(e));
            assert!(filter.allows(e));
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FilterError {
    #[error("failed to read event filters file {0}")]
    ReadFile(PathBuf, #[source] std::io::Error),

    #[error("failed to parse YAML event filters file {0}")]
    ParseYaml(PathBuf, #[source] serde_yml::Error),
}
