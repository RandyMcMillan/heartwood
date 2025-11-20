use super::*;

/// Trigger a CI run.
#[derive(Parser)]
pub struct TriggerCmd {
    /// Set the repository the event refers to. Can be a RID, or the
    /// repository name.
    #[clap(long)]
    repo: String,

    /// Set the name of the ref the event refers to.
    #[clap(long, alias = "ref")]
    name: String,

    /// Set the commit the event refers to. Can be the SHA1 commit id,
    /// or a symbolic Git revision, as understood by `git rev-parse`.
    /// For example, `HEAD`.
    #[clap(long)]
    commit: String,

    /// Write the event ID to this file, after adding the event to the
    /// queue.
    #[clap(long)]
    id_file: Option<PathBuf>,
}

impl Leaf for TriggerCmd {
    fn run(&self, args: &Args) -> Result<(), CibToolError> {
        let profile = util::load_profile()?;
        let nid = util::lookup_nid(&profile)?;
        let (rid, _repo_name) = util::lookup_repo(&profile, &self.repo)?;
        let oid = util::oid_from_cli_arg(&profile, rid, &self.commit)?;

        let base = util::lookup_commit(&profile, rid, &format!("{oid}^")).unwrap_or(oid);
        let name = format!("refs/namespaces/{nid}/refs/heads/{}", self.name.as_str());
        let name =
            RefString::try_from(name.clone()).map_err(|e| CibToolError::RefString(name, e))?;
        let event = CiEvent::branch_updated(nid, rid, &name, oid, base)
            .map_err(CibToolError::BranchCreted)?;

        let db = args.open_db()?;
        let id = db.push_queued_ci_event(event)?;
        println!("{id}");

        if let Some(filename) = &self.id_file {
            write(filename, id.to_string().as_bytes())
                .map_err(|e| CibToolError::WriteEventId(filename.into(), e))?;
        }

        Ok(())
    }
}
