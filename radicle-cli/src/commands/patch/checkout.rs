use anyhow::anyhow;

use radicle::cob::patch;
use radicle::cob::patch::RevisionId;
use radicle::git::RefString;
use radicle::storage::git::Repository;
use radicle::storage::ReadRepository;
use radicle::{git, rad};

use crate::terminal as term;

pub fn run(
    revision_id: &RevisionId,
    stored: &Repository,
    working: &git::raw::Repository,
) -> anyhow::Result<()> {
    let patches = patch::Patches::open(stored)?;

    let (patch_id, patch, _, revision) = patches
        .find_by_revision(revision_id)?
        .ok_or_else(|| anyhow!("Patch revision `{revision_id}` not found"))?;
    let (root, _) = patch.root();
    // If we passed in the root revision, it's more likely that the user was specifying the
    // patch itself. Hence, we checkout the latest update on the patch instead of that specific
    // revision.
    let revision = if *revision_id == root {
        let (_, revision) = patch.latest();
        revision
    } else {
        &revision
    };

    let mut spinner = term::spinner("Performing checkout...");
    let patch_branch =
        // SAFETY: Patch IDs are valid refstrings.
        git::refname!("patch").join(RefString::try_from(term::format::cob(&patch_id)).unwrap());

    match working.find_branch(patch_branch.as_str(), radicle::git::raw::BranchType::Local) {
        Ok(branch) => {
            let commit = branch.get().peel_to_commit()?;
            working.checkout_tree(commit.as_object(), None)?;
        }
        Err(e) if radicle::git::is_not_found_err(&e) => {
            let commit = find_patch_commit(revision, stored, working)?;
            // Create patch branch and switch to it.
            working.branch(patch_branch.as_str(), &commit, true)?;
            working.checkout_tree(commit.as_object(), None)?;
        }
        Err(e) => return Err(e.into()),
    }
    working.set_head(&git::refs::workdir::branch(&patch_branch))?;

    spinner.message(format!(
        "Switched to branch {}",
        term::format::highlight(&patch_branch)
    ));
    spinner.finish();

    if let Some(branch) = rad::setup_patch_upstream(&patch_id, revision.head(), working, false)? {
        let tracking = branch
            .name()?
            .ok_or_else(|| anyhow!("failed to create tracking branch: invalid name"))?;
        term::success!(
            "Branch {} setup to track {}",
            term::format::highlight(patch_branch),
            term::format::tertiary(tracking)
        );
    }
    Ok(())
}

/// Try to find the patch head in our working copy, and if we don't find it,
/// fetch it from storage first.
fn find_patch_commit<'a>(
    revision: &patch::Revision,
    stored: &Repository,
    working: &'a git::raw::Repository,
) -> anyhow::Result<git::raw::Commit<'a>> {
    let head = *revision.head();

    match working.find_commit(head) {
        Ok(commit) => Ok(commit),
        Err(e) if git::ext::is_not_found_err(&e) => {
            let url = git::url::File::new(stored.path());

            working.remote_anonymous(url.to_string().as_str())?.fetch(
                &[head.to_string()],
                None,
                None,
            )?;
            working.find_commit(head).map_err(|e| e.into())
        }
        Err(e) => Err(e.into()),
    }
}
