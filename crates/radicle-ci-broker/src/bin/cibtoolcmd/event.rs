use std::io::Write;

#[allow(unused_imports)] // FIXME
use radicle_ci_broker::{filter::EventFilter, node_event_source::NodeEventSource};

use super::*;

/// List events in the queue, waiting to be processed.
#[derive(Parser)]
pub struct ListEvents {
    /// Show more details about the event using Rust debug formatting.
    #[clap(long, hide = true)]
    verbose: bool,

    /// List events as JSON.
    #[clap(long)]
    json: bool,
}

impl Leaf for ListEvents {
    fn run(&self, args: &Args) -> Result<(), CibToolError> {
        let db = args.open_db()?;
        let event_ids = db.queued_ci_events()?;
        if self.json {
            let events: Result<Vec<QueuedCiEvent>, DbError> = event_ids
                .iter()
                .filter_map(|id| match db.get_queued_ci_event(id) {
                    Ok(Some(event)) => Some(Ok(event)),
                    Err(e) => Some(Err(e)),
                    _ => None,
                })
                .collect();
            let events = events?;
            let json =
                serde_json::to_string_pretty(&events).map_err(CibToolError::EventListToJson)?;
            println!("{}", json);
        } else if self.verbose {
            for id in event_ids {
                if let Some(e) = db.get_queued_ci_event(&id)? {
                    println!("{id}: {:?}", e);
                } else {
                    println!("{id}: No such event");
                }
            }
        } else {
            for id in event_ids {
                println!("{id}");
            }
        }
        Ok(())
    }
}

/// Show the number of events in the queue.
#[derive(Parser)]
pub struct CountEvents {}

impl Leaf for CountEvents {
    fn run(&self, args: &Args) -> Result<(), CibToolError> {
        let db = args.open_db()?;
        println!("{}", db.queued_ci_events()?.len());
        Ok(())
    }
}

/// Add an event to the queue.
#[derive(Parser)]
pub struct AddEvent {
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

    /// The base commit referred to by the event. The value is parsed
    /// the same way as `--commit` value.
    #[clap(long)]
    base: Option<String>,

    /// Write the event to this file, as JSON, instead of adding it to
    /// the queue.
    #[clap(long)]
    output: Option<PathBuf>,

    /// Write the event ID to this file, after adding the event to the
    /// queue.
    #[clap(long)]
    id_file: Option<PathBuf>,
}

impl AddEvent {
    fn lookup_nid(&self) -> Result<NodeId, CibToolError> {
        let profile = Profile::load().map_err(CibToolError::Profile)?;
        Ok(*profile.id())
    }

    fn lookup_rid(&self, wanted: &str) -> Result<RepoId, CibToolError> {
        let profile = Profile::load().map_err(CibToolError::Profile)?;
        let storage =
            Storage::open(profile.storage(), profile.info()).map_err(CibToolError::Storage)?;

        let mut rid = None;
        let repo_infos = storage.repositories().map_err(CibToolError::Repositories)?;
        for ri in repo_infos {
            let project = ri
                .doc
                .project()
                .map_err(|e| CibToolError::Project(ri.rid, e))?;

            if project.name() == wanted {
                if rid.is_some() {
                    return Err(CibToolError::DuplicateRepositories(wanted.into()));
                }
                rid = Some(ri.rid);
            }
        }

        if let Some(rid) = rid {
            Ok(rid)
        } else {
            Err(CibToolError::NotFound(wanted.into()))
        }
    }

    fn lookup_commit(&self, rid: RepoId, gitref: &str) -> Result<Oid, CibToolError> {
        let profile = Profile::load().map_err(CibToolError::Profile)?;
        let storage =
            Storage::open(profile.storage(), profile.info()).map_err(CibToolError::Storage)?;
        let repo = storage
            .repository(rid)
            .map_err(|e| CibToolError::RepoOpen(rid, e))?;
        let object = repo
            .backend
            .revparse_single(gitref)
            .map_err(|e| CibToolError::RevParse(gitref.into(), e))?;

        Ok(object.id().into())
    }
}

impl Leaf for AddEvent {
    fn run(&self, args: &Args) -> Result<(), CibToolError> {
        let nid = self.lookup_nid()?;

        let rid = if let Ok(rid) = RepoId::from_urn(&self.repo) {
            rid
        } else {
            self.lookup_rid(&self.repo)?
        };

        let oid = if let Ok(rid) = Oid::from_str(&self.commit) {
            rid
        } else {
            self.lookup_commit(rid, &self.commit)?
        };

        let name = format!("refs/namespaces/{}/refs/heads/{}", nid, self.name.as_str());
        let name =
            RefString::try_from(name.clone()).map_err(|e| CibToolError::RefString(name, e))?;

        let event = if let Some(base) = &self.base {
            let base = if let Ok(base) = Oid::from_str(base) {
                base
            } else {
                self.lookup_commit(rid, base)?
            };
            CiEvent::branch_updated(nid, rid, &name, oid, base)
                .map_err(CibToolError::BranchCreted)?
        } else {
            CiEvent::branch_created(nid, rid, &name, oid).map_err(CibToolError::BranchCreted)?
        };

        if let Some(output) = &self.output {
            let json = serde_json::to_string_pretty(&event)
                .map_err(|e| CibToolError::EventToJson(event.clone(), e))?;
            std::fs::write(output, json.as_bytes())
                .map_err(|e| CibToolError::Write(output.into(), e))?;
        } else {
            let db = args.open_db()?;
            let id = db.push_queued_ci_event(event)?;
            println!("{id}");

            if let Some(filename) = &self.id_file {
                write(filename, id.to_string().as_bytes())
                    .map_err(|e| CibToolError::WriteEventId(filename.into(), e))?;
            }
        }
        Ok(())
    }
}

/// Show an event in the queue.
#[derive(Parser)]
pub struct ShowEvent {
    /// ID of event to show.
    #[clap(long, required_unless_present = "id_file")]
    id: Option<QueueId>,

    /// Show event as JSON? Default is a debugging format useful for
    /// programmers.
    #[clap(long)]
    json: bool,

    /// Write output to this file.
    #[clap(long)]
    output: Option<PathBuf>,

    /// Read ID of event to show from this file.
    #[clap(long)]
    id_file: Option<PathBuf>,
}

impl Leaf for ShowEvent {
    fn run(&self, args: &Args) -> Result<(), CibToolError> {
        let db = args.open_db()?;

        let id = if let Some(id) = &self.id {
            id.clone()
        } else if let Some(filename) = &self.id_file {
            let id = read(filename).map_err(|e| CibToolError::ReadEventId(filename.into(), e))?;
            let id = String::from_utf8_lossy(&id).to_string();
            QueueId::from(&id)
        } else {
            return Err(CibToolError::MissingId);
        };

        if let Some(event) = db.get_queued_ci_event(&id)? {
            if self.json {
                let json = serde_json::to_string_pretty(&event.event())
                    .map_err(|e| CibToolError::EventToJson(event.event().clone(), e))?;
                if let Some(filename) = &self.output {
                    std::fs::write(filename, json.as_bytes())
                        .map_err(|e| CibToolError::Write(filename.into(), e))?;
                } else {
                    println!("{json}");
                }
            } else {
                println!("{event:#?}");
            }
        }
        Ok(())
    }
}

/// Remove an event from the queue.
#[derive(Parser)]
pub struct RemoveEvent {
    /// ID of event to remove.
    #[clap(long)]
    id: Option<QueueId>,

    /// Remove all queued events.
    #[clap(long)]
    all: bool,

    /// Read ID of event to remove from this file.
    #[clap(long)]
    id_file: Option<PathBuf>,
}

impl Leaf for RemoveEvent {
    fn run(&self, args: &Args) -> Result<(), CibToolError> {
        let db = args.open_db()?;

        let ids = if let Some(id) = &self.id {
            vec![id.clone()]
        } else if let Some(filename) = &self.id_file {
            let id = read(filename).map_err(|e| CibToolError::ReadEventId(filename.into(), e))?;
            let id = String::from_utf8_lossy(&id).to_string();
            vec![QueueId::from(&id)]
        } else if self.all {
            db.queued_ci_events()?
        } else {
            return Err(CibToolError::MissingId);
        };

        for id in ids {
            db.remove_queued_ci_event(&id)?;
        }
        Ok(())
    }
}

/// Add a shutdown event to the queue.
///
/// The shutdown event causes the CI broker to terminate.
#[derive(Parser)]
pub struct Shutdown {
    /// Write ID of the event to this file, after adding the event to
    /// the queue.
    #[clap(long)]
    id_file: Option<PathBuf>,
}

impl Leaf for Shutdown {
    fn run(&self, args: &Args) -> Result<(), CibToolError> {
        let db = args.open_db()?;
        let id = db.push_queued_ci_event(CiEvent::V1(CiEventV1::Shutdown))?;

        if let Some(filename) = &self.id_file {
            write(filename, id.to_string().as_bytes())
                .map_err(|e| CibToolError::WriteEventId(filename.into(), e))?;
        }

        Ok(())
    }
}

/// Record node events from the node.
///
/// The events are written to the standard output or to the specified
/// file, as one JSON object per line.
#[derive(Parser)]
pub struct RecordEvents {
    /// Write events to this file.
    #[clap(long)]
    output: Option<PathBuf>,
}

impl Leaf for RecordEvents {
    fn run(&self, _args: &Args) -> Result<(), CibToolError> {
        let profile = util::load_profile()?;
        let mut source = NodeEventSource::new(&profile).map_err(CibToolError::EventSubscribe)?;
        let mut file = if let Some(filename) = &self.output {
            Some(
                std::fs::File::create(filename)
                    .map_err(|e| CibToolError::CreateEventsFile(filename.into(), e))?,
            )
        } else {
            None
        };

        loop {
            match source.node_event() {
                Err(e) => return Err(CibToolError::GetNodeEvent(e)),
                Ok(None) => break,
                Ok(Some(event)) => {
                    println!("got event {event:?}");
                    let mut json = serde_json::to_string(&event)
                        .map_err(|e| CibToolError::NodeEevnetToJson(event.clone(), e))?;
                    if let Some(file) = &mut file {
                        json.push('\n');
                        file.write_all(json.as_bytes()).map_err(|e| {
                            CibToolError::WriteEvent(self.output.as_ref().unwrap().clone(), e)
                        })?;
                    } else {
                        println!("{json}");
                    }
                }
            }
        }

        Ok(())
    }
}

/// Convert node events into CI events.
///
/// Node events are read from the specified file. Note that one node
/// event can result in any number of broker events.
///
/// The CI events are written to the standard output or to the
/// specified file, as one JSON object per line.
#[derive(Parser)]
pub struct CiEvents {
    /// Write CI events to this file.
    #[clap(long)]
    output: Option<PathBuf>,

    /// Read node events from this file.
    input: PathBuf,
}

impl Leaf for CiEvents {
    fn run(&self, _args: &Args) -> Result<(), CibToolError> {
        let bytes = std::fs::read(&self.input)
            .map_err(|e| CibToolError::ReadEvents(self.input.clone(), e))?;
        let text = String::from_utf8(bytes)
            .map_err(|e| CibToolError::NodeEventNotUtf8(self.input.clone(), e))?;

        let mut node_events = vec![];
        for line in text.lines() {
            let event: radicle::node::Event = serde_json::from_str(line)
                .map_err(|e| CibToolError::JsonToNodeEvent(self.input.clone(), e))?;
            node_events.push(event);
        }

        let mut ci_events: Vec<CiEvent> = vec![];
        for node_event in node_events.iter() {
            if let Ok(mut cevs) = CiEvent::from_node_event(node_event) {
                ci_events.append(&mut cevs);
            }
        }

        let mut jsons = vec![];
        for ci_event in ci_events.iter() {
            jsons.push(
                serde_json::to_string(ci_event)
                    .map_err(|err| CibToolError::EventToJson(ci_event.clone(), err))?,
            );
        }

        if let Some(filename) = &self.output {
            let mut file = std::fs::File::create(filename)
                .map_err(|e| CibToolError::CreateBrokerEventsFile(filename.into(), e))?;
            for json in jsons {
                let json = format!("{json}\n");
                file.write_all(json.as_bytes()).map_err(|e| {
                    CibToolError::WriteEvent(self.output.as_ref().unwrap().clone(), e)
                })?;
            }
        } else {
            for json in jsons {
                println!("{json}");
            }
        }

        Ok(())
    }
}

/// Filter broker events recorded in a file.
///
/// Those broker events allowed by the filter are written to the
/// standard output, as one JSON object per line.
#[derive(Parser)]
pub struct FilterEvents {
    /// Read event filter from this YAML file.
    filters: PathBuf,

    /// Read broker events from this file.
    input: PathBuf,
}

impl Leaf for FilterEvents {
    fn run(&self, _args: &Args) -> Result<(), CibToolError> {
        let filters = EventFilter::from_file(&self.filters)
            .map_err(|e| CibToolError::ReadFilters(self.filters.clone(), e))?;

        let bytes = std::fs::read(&self.input)
            .map_err(|e| CibToolError::ReadBrokerEvents(self.input.clone(), e))?;
        let text = String::from_utf8(bytes)
            .map_err(|e| CibToolError::BrokerEventNotUtf8(self.input.clone(), e))?;

        let mut ci_events = vec![];
        for line in text.lines() {
            let event: CiEvent = serde_json::from_str(line)
                .map_err(|e| CibToolError::JsonToNodeEvent(self.input.clone(), e))?;
            ci_events.push(event);
        }

        for event in ci_events.iter() {
            for filter in filters.iter() {
                if filter.allows(event) {
                    let json = serde_json::to_string_pretty(event)
                        .map_err(|e| CibToolError::EventToJson(event.clone(), e))?;
                    println!("{json}");
                }
            }
        }

        Ok(())
    }
}
