//! Status and report pages for CI broker.
//!
//! This module generates an HTML status page for the CI broker, as
//! well as per-repository pages for any repository for which the CI
//! broker has mediated to run CI. The status page gives the latest
//! known status of the broker, plus lists the repositories that CI
//! has run for. The per-repository pages lists all the runs for that
//! repository.

use std::{
    collections::{HashMap, HashSet},
    fs::write,
    path::{Path, PathBuf},
    sync::mpsc::RecvTimeoutError,
    thread::{spawn, JoinHandle},
    time::Duration,
};

use html_page::{Element, HtmlPage, Tag};
use serde::Serialize;
use time::{macros::format_description, OffsetDateTime};

use radicle::{
    git::ext::Oid,
    prelude::RepoId,
    storage::{ReadRepository, ReadStorage},
    Profile,
};

use crate::{
    ci_event::{CiEvent, CiEventV1},
    db::{Db, DbError, QueuedCiEvent},
    logger,
    msg::RunId,
    notif::NotificationReceiver,
    run::{Run, RunState, Whence},
};

const CSS: &str = include_str!("radicle-ci.css");
const REFERESH_INTERVAL: &str = "300";
const UPDATE_INTERVAL: Duration = Duration::from_secs(60);
const STATUS_JSON: &str = "status.json";

/// All possible errors returned from the status page module.
#[derive(Debug, thiserror::Error)]
pub enum PageError {
    /// Error formatting a time as a string.
    #[error(transparent)]
    Timeformat(#[from] time::error::Format),

    #[error("failed to write status page to {0}")]
    Write(PathBuf, #[source] std::io::Error),

    #[error("no node alias has been set for builder")]
    NoAlias,

    #[error("no status data has been set for builder")]
    NoStatusData,

    #[error(transparent)]
    Db(#[from] DbError),

    #[error("failed to lock page data structure")]
    Lock(&'static str),

    #[error("failed to represent status page data as JSON")]
    StatusToJson(#[source] serde_json::Error),
}

fn now() -> Result<String, time::error::Format> {
    let fmt = format_description!("[year]-[month]-[day] [hour]:[minute]:[second]Z");
    OffsetDateTime::now_utc().format(fmt)
}

struct PageData {
    timestamp: String,
    ci_broker_version: &'static str,
    ci_broker_git_commit: &'static str,
    node_alias: String,
    runs: HashMap<RunId, Run>,
    events: Vec<QueuedCiEvent>,
    broker_event_counter: usize,
    latest_broker_event: Option<CiEvent>,
    latest_ci_run: Option<Run>,
}

impl PageData {
    fn status_page_as_json(&self) -> Result<String, PageError> {
        StatusData::from(self).as_json()
    }

    fn status_page_as_html(&self) -> Result<HtmlPage, PageError> {
        let mut doc = HtmlPage::default();

        let title = format!("CI for Radicle node {}", self.node_alias);
        Self::head(&mut doc, &title);

        doc.push_to_body(Element::new(Tag::H1).with_text(&title));

        Self::h1(&mut doc, "Broker status");
        doc.push_to_body(
            Element::new(Tag::P)
                .with_text("Last updated: ")
                .with_text(&self.timestamp)
                .with_child(Element::new(Tag::Br))
                .with_text("CI broker version: ")
                .with_text(self.ci_broker_version)
                .with_text(" (commit ")
                .with_child(Element::new(Tag::Code).with_text(self.ci_broker_git_commit))
                .with_text(")"),
        );

        Self::h1(&mut doc, "Repositories");
        Self::p_text(&mut doc, "Latest CI run for each repository.");

        let mut table = Element::new(Tag::Table).with_class("repolist").with_child(
            Element::new(Tag::Tr)
                .with_child(Element::new(Tag::Th).with_text("Repository"))
                .with_child(Element::new(Tag::Th).with_text("Run ID"))
                .with_child(Element::new(Tag::Th).with_text("Status"))
                .with_child(Element::new(Tag::Th).with_text("Info")),
        );

        for (alias, rid) in self.repos() {
            let (run_ids, status, info_url) = {
                let run = self.latest_run(rid);
                match run {
                    Some(run) => (
                        Self::run_ids(Some(run)),
                        Self::run_state(run),
                        Self::info_url(Some(run)),
                    ),
                    None => (
                        Self::run_ids(None),
                        Self::run_state_unknown(),
                        Self::info_url(None),
                    ),
                }
            };

            table.push_child(
                Element::new(Tag::Tr)
                    .with_child(Element::new(Tag::Td).with_child(Self::repository(rid, &alias)))
                    .with_child(Element::new(Tag::Td).with_child(run_ids))
                    .with_child(
                        Element::new(Tag::Td).with_child(
                            Element::new(Tag::Span)
                                .with_class("run-status")
                                .with_child(status),
                        ),
                    )
                    .with_child(Element::new(Tag::Td).with_child(info_url)),
            );
        }
        doc.push_to_body(table);

        Self::h1(&mut doc, "Event queue");
        let mut table = Element::new(Tag::Table)
            .with_class("event-queue")
            .with_child(
                Element::new(Tag::Tr)
                    .with_child(Element::new(Tag::Th).with_text("Queue id"))
                    .with_child(Element::new(Tag::Th).with_text("Timestamp"))
                    .with_child(Element::new(Tag::Th).with_text("Event")),
            );
        for event in self.events.iter() {
            fn render_event(repo: &RepoId, refname: &str, commit: &Oid) -> Element {
                Element::new(Tag::Span)
                    .with_child(
                        Element::new(Tag::Span)
                            .with_class("repoid")
                            .with_text(&repo.to_string()),
                    )
                    .with_child(Element::new(Tag::Br))
                    .with_child(Element::new(Tag::Span).with_class("ref").with_text(refname))
                    .with_child(Element::new(Tag::Br))
                    .with_child(
                        Element::new(Tag::Span)
                            .with_class("commit")
                            .with_text(&commit.to_string()),
                    )
            }

            let event_element = match event.event() {
                CiEvent::V1(CiEventV1::Shutdown) => Element::new(Tag::Span).with_text("shutdown"),
                CiEvent::V1(CiEventV1::BranchCreated {
                    from_node: _,
                    repo,
                    branch,
                    tip,
                }) => render_event(repo, branch, tip),
                CiEvent::V1(CiEventV1::BranchUpdated {
                    from_node: _,
                    repo,
                    branch,
                    tip,
                    old_tip: _,
                }) => render_event(repo, branch, tip),
                CiEvent::V1(CiEventV1::BranchDeleted {
                    repo, branch, tip, ..
                }) => render_event(repo, branch, tip),
                CiEvent::V1(CiEventV1::PatchCreated {
                    from_node: _,
                    repo,
                    patch,
                    new_tip,
                }) => render_event(repo, &patch.to_string(), new_tip),
                CiEvent::V1(CiEventV1::PatchUpdated {
                    from_node: _,
                    repo,
                    patch,
                    new_tip,
                }) => render_event(repo, &patch.to_string(), new_tip),
            };

            table.push_child(
                Element::new(Tag::Tr)
                    .with_child(Element::new(Tag::Td).with_text(&event.id().to_string()))
                    .with_child(Element::new(Tag::Td).with_text(event.timestamp()))
                    .with_child(Element::new(Tag::Td).with_child(event_element)),
            );
        }
        doc.push_to_body(table);

        Self::h1(&mut doc, "Recent status");
        let status = StatusData::from(self).as_json()?;
        doc.push_to_body(
            Element::new(Tag::P)
                .with_text("See also as a separate file ")
                .with_child(
                    Element::new(Tag::A)
                        .with_attribute("href", STATUS_JSON)
                        .with_child(Element::new(Tag::Code).with_text(STATUS_JSON)),
                )
                .with_text(": "),
        );
        doc.push_to_body(
            Element::new(Tag::Blockquote).with_child(Element::new(Tag::Pre).with_text(&status)),
        );

        Ok(doc)
    }

    fn run_state(run: &Run) -> Element {
        let status = match run.state() {
            RunState::Finished => {
                if let Some(result) = run.result() {
                    format!("{}, {}", run.state(), result)
                } else {
                    format!("{} with unknown result", run.state())
                }
            }
            _ => run.state().to_string(),
        };

        Element::new(Tag::Span)
            .with_class(&status)
            .with_text(&status.to_string())
    }

    fn run_state_unknown() -> Element {
        Element::new(Tag::Span)
            .with_class("run-status")
            .with_text("unknown")
    }

    fn per_repo_page_as_html(&self, rid: RepoId, alias: &str, timestamp: &str) -> HtmlPage {
        let mut doc = HtmlPage::default();

        let title = format!("CI runs for repository {}", alias);
        Self::head(&mut doc, &title);

        Self::h1(&mut doc, &title);
        doc.push_to_body(
            Element::new(Tag::P).with_text("Repository ID ").with_child(
                Element::new(Tag::Code)
                    .with_class("repoid")
                    .with_text(&rid.to_string()),
            ),
        );
        Self::p_text(&mut doc, &format!("Last updated: {timestamp}"));

        let mut table = Element::new(Tag::Table).with_class("run-list").with_child(
            Element::new(Tag::Tr)
                .with_child(Element::new(Tag::Th).with_text("Run ID"))
                .with_child(Element::new(Tag::Th).with_text("Whence"))
                .with_child(Element::new(Tag::Th).with_text("Status"))
                .with_child(Element::new(Tag::Th).with_text("Info")),
        );

        let mut runs = self.runs(rid);
        runs.sort_by_cached_key(|run| run.timestamp());
        runs.reverse();
        for run in runs {
            let current = match run.state() {
                RunState::Triggered => Element::new(Tag::Span)
                    .with_attribute("state", "triggered")
                    .with_text("triggered"),
                RunState::Running => Element::new(Tag::Span)
                    .with_class("running)")
                    .with_text("running"),
                RunState::Finished => {
                    let result = if let Some(result) = run.result() {
                        result.to_string()
                    } else {
                        "unknown".into()
                    };
                    Element::new(Tag::Span)
                        .with_class(&result)
                        .with_text(&result)
                }
            };

            table.push_child(
                Element::new(Tag::Tr)
                    .with_child(Element::new(Tag::Td).with_child(Self::run_ids(Some(run))))
                    .with_child(
                        Element::new(Tag::Td).with_child(Self::whence_as_html(run.whence())),
                    )
                    .with_child(Element::new(Tag::Td).with_child(current))
                    .with_child(Element::new(Tag::Td).with_child(Self::info_url(Some(run)))),
            );
        }

        doc.push_to_body(table);

        doc
    }

    fn head(page: &mut HtmlPage, title: &str) {
        page.push_to_head(Element::new(Tag::Title).with_text(title));
        page.push_to_head(Element::new(Tag::Style).with_text(CSS));
        page.push_to_head(
            Element::new(Tag::Meta)
                .with_attribute("http-equiv", "refresh")
                .with_attribute("content", REFERESH_INTERVAL),
        );
    }

    fn h1(page: &mut HtmlPage, text: &str) {
        page.push_to_body(Element::new(Tag::H2).with_text(text));
    }

    fn p_text(page: &mut HtmlPage, text: &str) {
        page.push_to_body(Element::new(Tag::P).with_text(text));
    }

    fn repository(repo_id: RepoId, alias: &str) -> Element {
        Element::new(Tag::A)
            .with_child(
                Element::new(Tag::Code)
                    .with_class("alias)")
                    .with_text(alias),
            )
            .with_attribute("href", &format!("{}.html", rid_to_basename(repo_id)))
    }

    fn run_ids(run: Option<&Run>) -> Element {
        if let Some(run) = run {
            let adapter_run_id = if let Some(x) = run.adapter_run_id() {
                Element::new(Tag::Span).with_text(x.as_str())
            } else {
                Element::new(Tag::Span)
            };

            Element::new(Tag::Span).with_child(
                Element::new(Tag::Span)
                    .with_class("adapter-run-id")
                    .with_child(adapter_run_id)
                    .with_child(Element::new(Tag::Br))
                    .with_child(
                        Element::new(Tag::Span)
                            .with_class("broker-run-id")
                            .with_text(&run.broker_run_id().to_string()),
                    )
                    .with_child(Element::new(Tag::Br))
                    .with_text(run.timestamp()),
            )
        } else {
            Element::new(Tag::Span)
        }
    }

    fn info_url(run: Option<&Run>) -> Element {
        if let Some(run) = run {
            if let Some(url) = run.adapter_info_url() {
                return Element::new(Tag::A)
                    .with_attribute("href", url)
                    .with_text("info");
            }
        }
        Element::new(Tag::Span)
    }

    fn whence_as_html(whence: &Whence) -> Element {
        match whence {
            Whence::Branch {
                name,
                commit,
                who: _,
            } => Element::new(Tag::Span)
                .with_text("branch ")
                .with_child(
                    Element::new(Tag::Code)
                        .with_class("branch)")
                        .with_text(name),
                )
                .with_text(", commit  ")
                .with_child(
                    Element::new(Tag::Code)
                        .with_class("commit)")
                        .with_text(&commit.to_string()),
                )
                .with_child(Element::new(Tag::Br))
                .with_text("from ")
                .with_child(
                    Element::new(Tag::Span)
                        .with_class("who")
                        .with_text(whence.who().unwrap_or("<commit author not known>")),
                ),
            Whence::Patch {
                patch,
                commit,
                revision,
                who: _,
            } => Element::new(Tag::Span)
                .with_text("patch ")
                .with_child(
                    Element::new(Tag::Code)
                        .with_class("branch")
                        .with_text(&patch.to_string()),
                )
                .with_child(Element::new(Tag::Br))
                .with_text("revision ")
                .with_child(Element::new(Tag::Code).with_class("revision)").with_text(&{
                    if let Some(rev) = &revision {
                        rev.to_string()
                    } else {
                        "<unknown patch revision>".to_string()
                    }
                }))
                .with_child(Element::new(Tag::Br))
                .with_text("commit ")
                .with_child(
                    Element::new(Tag::Code)
                        .with_class("commit)")
                        .with_text(&commit.to_string()),
                )
                .with_child(Element::new(Tag::Br))
                .with_text("from ")
                .with_child(
                    Element::new(Tag::Span)
                        .with_class("who")
                        .with_text(whence.who().unwrap_or("<patch author not known>")),
                ),
        }
    }

    fn repos(&self) -> Vec<(String, RepoId)> {
        let rids: HashSet<(String, RepoId)> = self
            .runs
            .values()
            .map(|run| (run.repo_alias().to_string(), run.repo_id()))
            .collect();
        let mut repos: Vec<(String, RepoId)> = rids.iter().cloned().collect();
        repos.sort();
        repos
    }

    fn repo_alias(&self, wanted: RepoId) -> Option<String> {
        self.repos().iter().find_map(|(alias, rid)| {
            if *rid == wanted {
                Some(alias.into())
            } else {
                None
            }
        })
    }

    fn runs(&self, repoid: RepoId) -> Vec<&Run> {
        self.runs
            .iter()
            .filter_map(|(_, run)| {
                if run.repo_id() == repoid {
                    Some(run)
                } else {
                    None
                }
            })
            .collect()
    }

    fn latest_run(&self, repoid: RepoId) -> Option<&Run> {
        let mut value: Option<&Run> = None;
        for run in self.runs(repoid) {
            if let Some(latest) = value {
                if run.timestamp() > latest.timestamp() {
                    value = Some(run);
                }
            } else {
                value = Some(run);
            }
        }
        value
    }
}

/// Data for status pages for CI broker.
///
/// There is a "front page" with status about the broker, and a list
/// of repositories for which the broker has run CI. Then there is a
/// page per such repository, with a list of CI runs for that
/// repository.
#[derive(Default)]
pub struct StatusPage {
    node_alias: String,
    dirname: Option<PathBuf>,
}

impl StatusPage {
    pub fn set_output_dir(&mut self, dirname: &Path) {
        self.dirname = Some(dirname.into());
    }

    pub fn update_in_thread(
        mut self,
        run_rx: NotificationReceiver,
        profile: Profile,
        db: Db,
        once: bool,
    ) -> JoinHandle<Result<(), PageError>> {
        logger::pages_start();

        if self.dirname.is_none() {
            logger::pages_directory_unset();
        }

        self.node_alias = profile.config.alias().to_string();
        logger::pages_interval(UPDATE_INTERVAL);

        spawn(move || {
            'processing_loop: loop {
                self.update_and_write(&profile, &db)?;
                if once {
                    return Ok(());
                }

                match run_rx.wait_for_notification() {
                    Ok(_) => (),
                    Err(RecvTimeoutError::Timeout) => (),
                    Err(RecvTimeoutError::Disconnected) => {
                        logger::pages_disconnected();
                        break 'processing_loop;
                    }
                }
            }

            // Make sure we update reports and status JSON at least once.
            self.update_and_write(&profile, &db)?;

            logger::pages_end();
            Ok(())
        })
    }

    fn update_and_write(&mut self, profile: &Profile, db: &Db) -> Result<(), PageError> {
        if let Some(dirname) = &self.dirname {
            let runs = db.get_all_runs()?;

            // Create list of events, except ones for private
            // repositories.
            let events: Result<Vec<QueuedCiEvent>, PageError> = db
                .queued_ci_events()?
                .iter()
                .filter_map(|id| match db.get_queued_ci_event(id) {
                    Ok(Some(event)) => match event.event() {
                        CiEvent::V1(CiEventV1::Shutdown) => Some(Ok(event)),
                        CiEvent::V1(CiEventV1::BranchCreated { repo, .. })
                        | CiEvent::V1(CiEventV1::BranchUpdated { repo, .. })
                        | CiEvent::V1(CiEventV1::PatchCreated { repo, .. })
                        | CiEvent::V1(CiEventV1::PatchUpdated { repo, .. }) => {
                            if Self::is_public_repo(profile, repo) {
                                Some(Ok(event))
                            } else {
                                None
                            }
                        }
                        _ => None,
                    },
                    Ok(None) => None, // Event is (no longer?) in database.
                    Err(_) => None,   // We ignore error here on purpose.
                })
                .collect();
            let mut events = events?;
            events.sort_by_cached_key(|e| e.timestamp().to_string());

            let data = PageData {
                timestamp: now()?,
                ci_broker_version: env!("CARGO_PKG_VERSION"),
                ci_broker_git_commit: env!("GIT_HEAD"),
                node_alias: self.node_alias.clone(),
                runs: HashMap::from_iter(
                    runs.iter()
                        .map(|run| (run.broker_run_id().clone(), run.clone())),
                ),
                events,
                broker_event_counter: 0,
                latest_broker_event: None,
                latest_ci_run: None,
            };

            let nameless = String::from("nameless repo");

            // We avoid writing while keeping the lock, to reduce
            // contention.
            let (status, repos) = {
                let status = data.status_page_as_html()?.to_string();

                let mut repos = vec![];
                for (_, rid) in data.repos() {
                    let basename = rid_to_basename(rid);
                    let filename = dirname.join(format!("{basename}.html"));
                    let alias = data.repo_alias(rid).unwrap_or(nameless.clone());
                    let repopage = data.per_repo_page_as_html(rid, &alias, &data.timestamp);
                    repos.push((filename, repopage.to_string()));
                }

                (status, repos)
            };

            let filename = dirname.join("index.html");
            Self::write_file(&filename, &status)?;

            for (filename, repopage) in repos {
                Self::write_file(&filename, &repopage)?;
            }

            let filename = dirname.join(STATUS_JSON);
            let json = data.status_page_as_json()?;
            Self::write_file(&filename, &json)?;
        }
        Ok(())
    }

    fn is_public_repo(profile: &Profile, rid: &RepoId) -> bool {
        if let Ok(repo) = profile.storage.repository(*rid) {
            if let Ok(id_doc) = repo.canonical_identity_doc() {
                if id_doc.doc.visibility.is_public() {
                    return true;
                }
            }
        }
        false
    }

    fn write_file(filename: &Path, text: &str) -> Result<(), PageError> {
        write(filename, text).map_err(|e| PageError::Write(filename.into(), e))?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
struct StatusData {
    timestamp: String,
    broker_event_counter: usize,
    ci_broker_version: &'static str,
    ci_broker_git_commit: &'static str,
    latest_broker_event: Option<CiEvent>,
    latest_ci_run: Option<Run>,
    event_queue_length: usize,
}

impl StatusData {
    fn as_json(&self) -> Result<String, PageError> {
        serde_json::to_string_pretty(self).map_err(PageError::StatusToJson)
    }
}

impl From<&PageData> for StatusData {
    fn from(page: &PageData) -> Self {
        Self {
            timestamp: page.timestamp.clone(),
            broker_event_counter: page.broker_event_counter,
            ci_broker_version: page.ci_broker_version,
            ci_broker_git_commit: page.ci_broker_git_commit,
            latest_broker_event: page.latest_broker_event.clone(),
            latest_ci_run: page.latest_ci_run.clone(),
            event_queue_length: page.events.len(),
        }
    }
}

fn rid_to_basename(repoid: RepoId) -> String {
    let mut basename = repoid.to_string();
    assert!(basename.starts_with("rad:"));
    basename.drain(..4);
    basename
}
