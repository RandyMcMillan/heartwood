//! The datadir step library provides a small number of steps which can be used
//! in your scenarios, though its primary use is to provide the Datadir context
//! object which other libraries will use to provide filesystem access.
//!
//! If you want to create files, run commands, etc. from your scenarios, you
//! will likely be using them from within the datadir.

use std::fmt::Debug;
use std::fs::{File, OpenOptions};
use std::path::{Component, Path, PathBuf};

pub use crate::prelude::*;

/// The `Datadir` is the context type which provides a directory for each scenario
/// and allows for the creation and testing of files within that directory.
///
/// A few steps are provided as part of this step library, though in reality the
/// majority of steps which interact with the data directory are in the
/// [files][`crate::steplibrary::files`] step library, and commands which interact
/// with stuff in here are in the [runcmd][`crate::steplibrary::runcmd`] step library.
///
#[derive(Default)]
pub struct Datadir {
    inner: Option<DatadirInner>,
}

#[derive(Debug)]
pub struct DatadirInner {
    base: tempfile::TempDir,
}

impl Debug for Datadir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.inner {
            Some(inner) => (inner as &dyn Debug).fmt(f),
            None => f
                .debug_struct("Datadir")
                .field("inner", &self.inner)
                .finish(),
        }
    }
}

impl ContextElement for Datadir {
    fn created(&mut self, scenario: &Scenario) {
        assert!(self.inner.is_none());
        self.inner = DatadirInner::build(scenario.title());
    }

    // TODO: Support saving datadir between steps
}

impl DatadirInner {
    fn build(title: &str) -> Option<Self> {
        let safe_title: String = title
            .chars()
            .map(|c| match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' => c,
                _ => '-',
            })
            .collect();
        let base = tempfile::Builder::new()
            .prefix("subplot")
            .suffix(&safe_title)
            .rand_bytes(5)
            .tempdir()
            .ok()?;

        Some(Self { base })
    }
}

impl Datadir {
    fn inner(&self) -> &DatadirInner {
        self.inner
            .as_ref()
            .expect("Attempted to access Datadir too early (or late)")
    }

    /// Retrieve the base data directory path which can be used to store
    /// files etc. for this step.
    ///
    /// This is used by steps wishing to manipulate the content of the data directory.
    pub fn base_path(&self) -> &Path {
        self.inner().base.path()
    }

    /// Canonicalise a subpath into this dir
    ///
    /// This step **safely** joins the base path to the given subpath.  This ensures that,
    /// for example, the subpath is relative, does not contain `..` elements, etc.
    #[throws(StepError)]
    pub fn canonicalise_filename<S: AsRef<Path>>(&self, subpath: S) -> PathBuf {
        let mut ret = self.base_path().to_path_buf();
        for component in subpath.as_ref().components() {
            match component {
                Component::CurDir => {}
                Component::ParentDir => {
                    throw!("embedded filenames may not contain ..");
                }
                Component::RootDir | Component::Prefix(_) => {
                    throw!("embedded filenames must be relative");
                }
                c => ret.push(c),
            }
        }
        ret
    }

    /// Open a file for writing
    ///
    /// This is a convenience function to open a file for writing at the given subpath.
    #[throws(StepError)]
    pub fn open_write<S: AsRef<Path>>(&self, subpath: S) -> File {
        let full_path = self.canonicalise_filename(subpath)?;
        OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(full_path)?
    }

    /// Open a file for reading
    ///
    /// This is a convenience function to open a file for reading from the given subpath
    #[throws(StepError)]
    pub fn open_read<S: AsRef<Path>>(&self, subpath: S) -> File {
        let full_path = self.canonicalise_filename(subpath)?;
        OpenOptions::new()
            .create(false)
            .write(false)
            .read(true)
            .open(full_path)?
    }

    /// Make a directory tree
    ///
    /// Equivalent to `mkdir -p` this will create the full path to the given subpath
    /// allowing subsequent step code to use that directory.
    #[throws(StepError)]
    pub fn create_dir_all<S: AsRef<Path>>(&self, subpath: S) {
        let full_path = self.canonicalise_filename(subpath)?;
        std::fs::create_dir_all(full_path)?;
    }
}

/// A simple check for enough disk space in the data dir
#[step]
pub fn datadir_has_enough_space(datadir: &Datadir, bytes: u64) {
    let available = fs2::available_space(datadir.base_path())?;
    if available < bytes {
        throw!(format!(
            "Available space check failed, wanted {bytes} bytes, but only {available} were available",
        ));
    }
}

/// A convenience step for enough disk space in the data dir in megabytes
#[step]
pub fn datadir_has_enough_space_megabytes(context: &ScenarioContext, megabytes: u64) {
    datadir_has_enough_space::call(context, megabytes * 1024 * 1024)?;
}
