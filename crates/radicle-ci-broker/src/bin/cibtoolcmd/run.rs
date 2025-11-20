use super::*;

#[derive(Parser)]
pub struct AddRun {
    /// Set the adapter run ID.
    #[clap(long)]
    id: Option<RunId>,

    /// Set optional URL to information about the CI run.
    #[clap(long)]
    url: Option<String>,

    /// Set the repository the event refers to. Can be a RID, or the
    /// repository name.
    #[clap(long)]
    repo: String,

    /// Set timestamp of the CI run.
    #[clap(long)]
    timestamp: Option<String>,

    /// Set the Git branch used by the CI run.
    #[clap(long)]
    branch: String,

    /// Set the commit SHA ID used by the CI run.
    #[clap(long)]
    commit: String,

    /// Set the author of the commit used by the CI run.
    #[clap(long)]
    who: Option<String>,

    /// Set the state of the CI run to "triggered".
    #[clap(long, required_unless_present_any = ["running", "success", "failure"])]
    triggered: bool,

    /// Set the state of the CI run to "running".
    #[clap(long)]
    #[clap(long, required_unless_present_any = ["triggered", "success", "failure"])]
    running: bool,

    /// Mark the CI run as finished and successful.
    #[clap(long, required_unless_present_any = ["triggered", "running", "failure"])]
    success: bool,

    /// Mark the CI run as finished and failed.
    #[clap(long, required_unless_present_any = ["triggered", "running", "success"])]
    failure: bool,
}

impl Leaf for AddRun {
    fn run(&self, args: &Args) -> Result<(), CibToolError> {
        let db = args.open_db()?;

        let profile = util::load_profile()?;
        let (rid, repo_name) = util::lookup_repo(&profile, &self.repo)?;
        let oid = util::oid_from_cli_arg(&profile, rid, &self.commit)?;
        let ts = self.timestamp.clone().unwrap_or(util::now()?);

        let whence = Whence::Branch {
            name: self.branch.clone(),
            commit: oid,
            who: self.who.clone(),
        };
        let mut run = Run::new(rid, &repo_name, whence, ts);

        let id = self.id.clone().unwrap_or_default();
        run.set_adapter_run_id(id);

        if let Some(url) = &self.url {
            run.set_adapter_info_url(url);
        }

        if self.triggered {
            run.set_state(RunState::Triggered);
            run.unset_result();
        } else if self.running {
            run.set_state(RunState::Running);
            run.unset_result();
        } else if self.success {
            run.set_result(RunResult::Success);
            run.set_state(RunState::Finished);
        } else if self.failure {
            run.set_result(RunResult::Failure);
            run.set_state(RunState::Finished);
        } else {
            return Err(CibToolError::AddRunConfusion);
        }

        db.push_run(&run)?;

        Ok(())
    }
}

#[derive(Parser)]
pub struct UpdateRun {
    /// ID of run to update.
    #[clap(long)]
    id: RunId,

    /// Set the state of the CI run to "triggered".
    #[clap(long, required_unless_present_any = ["running", "success", "failure"])]
    triggered: bool,

    /// Set the state of the CI run to "running".
    #[clap(long)]
    #[clap(long, required_unless_present_any = ["triggered", "success", "failure"])]
    running: bool,

    /// Mark the CI run as finished and successful.
    #[clap(long, required_unless_present_any = ["triggered", "running", "failure"])]
    success: bool,

    /// Mark the CI run as finished and failed.
    #[clap(long, required_unless_present_any = ["triggered", "running", "success"])]
    failure: bool,
}

impl Leaf for UpdateRun {
    fn run(&self, args: &Args) -> Result<(), CibToolError> {
        let db = args.open_db()?;

        let runs = db.find_runs(&self.id)?;
        if runs.is_empty() {
            Err(CibToolError::RunNotFound(self.id.clone()))
        } else {
            let mut run = runs.first().unwrap().clone(); // this is safe: runs is not empty

            run.unset_result();
            if self.triggered {
                run.set_state(RunState::Triggered);
            } else if self.running {
                run.set_state(RunState::Running);
            } else if self.success {
                run.set_state(RunState::Finished);
                run.set_result(RunResult::Success);
            } else if self.failure {
                run.set_state(RunState::Finished);
                run.set_result(RunResult::Failure);
            }

            db.update_run(&run)?;
            Ok(())
        }
    }
}

#[derive(Parser)]
pub struct ShowRun {
    /// Broker or adapter run IC.
    id: RunId,
}

impl Leaf for ShowRun {
    fn run(&self, args: &Args) -> Result<(), CibToolError> {
        let db = args.open_db()?;
        let runs = db.find_runs(&self.id)?;
        let json = serde_json::to_string_pretty(&runs).map_err(CibToolError::RunToJson)?;
        println!("{json}");

        Ok(())
    }
}

#[derive(Parser)]
pub struct ListRuns {
    #[clap(long)]
    json: bool,

    #[clap(long)]
    adapter_run_id: Option<RunId>,
}

impl Leaf for ListRuns {
    fn run(&self, args: &Args) -> Result<(), CibToolError> {
        let db = args.open_db()?;

        let runs = if let Some(wanted) = &self.adapter_run_id {
            db.find_runs(wanted)?
        } else {
            db.get_all_runs()?
        };

        if self.json {
            println!(
                "{}",
                serde_json::to_string_pretty(&runs).map_err(CibToolError::RunToJson)?
            );
        } else {
            for run in runs {
                println!(
                    "{} {}",
                    run.broker_run_id(),
                    run.adapter_run_id()
                        .map(|id| id.to_string())
                        .unwrap_or("unknown".into())
                );
            }
        }

        Ok(())
    }
}

#[derive(Serialize)]
struct RunInfo {
    run_id: String,
    timestamp: String,
    repo_id: String,
    repo_alias: String,
    whence: Whence,
    info_url: Option<String>,
    state: RunState,
    result: Option<RunResult>,
}

impl From<&Run> for RunInfo {
    fn from(run: &Run) -> Self {
        RunInfo {
            run_id: run
                .adapter_run_id()
                .map(|id| id.to_string())
                .unwrap_or("unknown".into()),
            timestamp: run.timestamp().into(),
            repo_id: run.repo_id().to_string(),
            repo_alias: run.repo_alias().into(),
            whence: run.whence().clone(),
            info_url: run.adapter_info_url().map(|url| url.into()),
            state: run.state(),
            result: run.result().cloned(),
        }
    }
}
