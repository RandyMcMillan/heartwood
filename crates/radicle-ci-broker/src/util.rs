use std::str::FromStr;

use time::{macros::format_description, OffsetDateTime};

use radicle::{
    prelude::{NodeId, RepoId},
    storage::ReadStorage,
    Profile, Storage,
};
use radicle_git_ext::Oid;

pub fn lookup_repo(profile: &Profile, wanted: &str) -> Result<(RepoId, String), UtilError> {
    let storage = Storage::open(profile.storage(), profile.info()).map_err(UtilError::Storage)?;

    let repos = storage.repositories().map_err(UtilError::Repositories)?;
    let mut rid = None;

    if let Ok(wanted_rid) = RepoId::from_urn(wanted) {
        for ri in repos {
            let project = ri
                .doc
                .project()
                .map_err(|e| UtilError::Project(ri.rid, e))?;

            if ri.rid == wanted_rid {
                if rid.is_some() {
                    return Err(UtilError::DuplicateRepositories(wanted.into()));
                }
                rid = Some((ri.rid, project.name().to_string()));
            }
        }
    } else {
        for ri in repos {
            let project = ri
                .doc
                .project()
                .map_err(|e| UtilError::Project(ri.rid, e))?;

            if project.name() == wanted {
                if rid.is_some() {
                    return Err(UtilError::DuplicateRepositories(wanted.into()));
                }
                rid = Some((ri.rid, project.name().to_string()));
            }
        }
    }

    if let Some(rid) = rid {
        Ok(rid)
    } else {
        Err(UtilError::NotFound(wanted.into()))
    }
}

pub fn oid_from_cli_arg(profile: &Profile, rid: RepoId, commit: &str) -> Result<Oid, UtilError> {
    if let Ok(oid) = Oid::from_str(commit) {
        Ok(oid)
    } else {
        lookup_commit(profile, rid, commit)
    }
}

pub fn load_profile() -> Result<Profile, UtilError> {
    Profile::load().map_err(UtilError::Profile)
}

pub fn lookup_nid(profile: &Profile) -> Result<NodeId, UtilError> {
    Ok(*profile.id())
}

pub fn lookup_commit(profile: &Profile, rid: RepoId, gitref: &str) -> Result<Oid, UtilError> {
    let storage = Storage::open(profile.storage(), profile.info()).map_err(UtilError::Storage)?;
    let repo = storage
        .repository(rid)
        .map_err(|e| UtilError::RepoOpen(rid, e))?;
    let object = repo
        .backend
        .revparse_single(gitref)
        .map_err(|e| UtilError::RevParse(gitref.into(), e))?;

    Ok(object.id().into())
}

pub fn now() -> Result<String, UtilError> {
    let fmt = format_description!("[year]-[month]-[day] [hour]:[minute]:[second]Z");
    OffsetDateTime::now_utc()
        .format(fmt)
        .map_err(UtilError::TimeFormat)
}

#[derive(Debug, thiserror::Error)]
pub enum UtilError {
    #[error("failed to look up node profile")]
    Profile(#[source] radicle::profile::Error),

    #[error("failed to look up open node storage")]
    Storage(#[source] radicle::storage::Error),

    #[error("failed to list repositories in node storage")]
    Repositories(#[source] radicle::storage::Error),

    #[error("failed to look up project info for repository {0}")]
    Project(RepoId, #[source] radicle::identity::doc::PayloadError),

    #[error("node has more than one repository called {0}")]
    DuplicateRepositories(String),

    #[error("node has no repository called: {0}")]
    NotFound(String),

    #[error("failed to open git repository in node storage: {0}")]
    RepoOpen(RepoId, #[source] radicle::storage::RepositoryError),

    #[error("failed to parse git ref as a commit id: {0}")]
    RevParse(String, #[source] radicle::git::raw::Error),

    #[error("failed to format time stamp")]
    TimeFormat(#[source] time::error::Format),
}
