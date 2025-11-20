//! Library of steps for handling files in the data dir.
//!
//! The files step library is intended to help with standard operations which
//! people might need when writing subplot scenarios which use embedded files.

use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::fs::{self, Metadata, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use filetime::FileTime;
use regex::Regex;
use time::macros::format_description;
use time::OffsetDateTime;

pub use crate::prelude::*;

pub use super::datadir::Datadir;

#[derive(Debug, Default)]
/// Context data for the `files` step library
///
/// This context contains a mapping from filename to metadata so that
/// the various steps remember metadata and then query it later can find it.
///
/// This context depends on, and will automatically register, the context for
/// the [`datadir`][crate::steplibrary::datadir] step library.
///
/// Because files can typically only be named in Subplot documents, we assume they
/// all have names which can be rendered as utf-8 strings.
pub struct Files {
    metadata: HashMap<PathBuf, Metadata>,
}

impl ContextElement for Files {
    fn created(&mut self, scenario: &Scenario) {
        scenario.register_context_type::<Datadir>();
    }
}

/// Create a file on disk from an embedded file
///
/// # `given file {embedded_file}`
///
/// Create a file in the data dir from an embedded file.
///
/// This defers to [`create_from_embedded_with_other_name`]
#[step]
#[context(Datadir)]
pub fn create_from_embedded(context: &ScenarioContext, embedded_file: SubplotDataFile) {
    let filename_on_disk = PathBuf::from(format!("{}", embedded_file.name().display()));
    create_from_embedded_with_other_name::call(context, &filename_on_disk, embedded_file)?;
}

/// Create a file on disk from an embedded file with a given name
///
/// # `given file {filename_on_disk} from {embedded_file}`
///
/// Creates a file in the data dir from an embedded file, but giving it a
/// potentially different name.
#[step]
pub fn create_from_embedded_with_other_name(
    context: &Datadir,
    filename_on_disk: &Path,
    embedded_file: SubplotDataFile,
) {
    let filename_on_disk = PathBuf::from(filename_on_disk);
    let parentpath = filename_on_disk.parent().ok_or_else(|| {
        format!(
            "No parent directory found for {}",
            filename_on_disk.display()
        )
    })?;
    context.create_dir_all(parentpath)?;
    context
        .open_write(filename_on_disk)?
        .write_all(embedded_file.data())?;
}

/// Touch a file to have a specific timestamp as its modified time
///
/// # `given file (?P<filename>\S+) has modification time (?P<mtime>\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2})`
///
/// Sets the modification time for the given filename to the provided mtime.
/// If the file does not exist, it will be created.
#[step]
pub fn touch_with_timestamp(context: &Datadir, filename: &Path, mtime: &str) {
    let fd = format_description!(
        "[year]-[month]-[day] [hour]:[minute]:[second] [offset_hour]:[offset_minute]"
    );
    let full_time = format!("{mtime} +00:00");
    let ts = OffsetDateTime::parse(&full_time, &fd)?;
    let (secs, nanos) = (ts.unix_timestamp(), 0);
    let mtime = FileTime::from_unix_time(secs, nanos);
    let full_path = context.canonicalise_filename(filename)?;
    // If the file doesn't exist, create it
    drop(
        OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&full_path)?,
    );
    // And set its mtime
    filetime::set_file_mtime(full_path, mtime)?;
}

/// Create a file with some given text as its content
///
/// # `when I write "(?P<text>.*)" to file (?P<filename>\S+)`
///
/// Create/replace the given file with the given content.
#[step]
pub fn create_from_text(context: &Datadir, text: &str, filename: &Path) {
    context.open_write(filename)?.write_all(text.as_bytes())?;
}

/// Examine the given file and remember its metadata for later
///
/// # `when I remember metadata for file {filename}`
///
/// This step stores the metadata (mtime etc) for the given file into the
/// context so that it can be retrieved later for testing against.
#[step]
#[context(Datadir)]
#[context(Files)]
pub fn remember_metadata(context: &ScenarioContext, filename: &Path) {
    let full_path = context.with(
        |context: &Datadir| context.canonicalise_filename(filename),
        false,
    )?;
    let metadata = fs::metadata(full_path)?;
    context.with_mut(
        |context: &mut Files| {
            context.metadata.insert(filename.to_owned(), metadata);
            Ok(())
        },
        false,
    )?;
}

/// Touch a given file
///
/// # `when I touch file {filename}`
///
/// This will create the named file if it does not exist, and then it will ensure that the
/// file's modification time is set to the current time.
#[step]
pub fn touch(context: &Datadir, filename: &Path) {
    let full_path = context.canonicalise_filename(filename)?;
    let now = FileTime::now();
    // If the file doesn't exist, create it
    drop(
        OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&full_path)?,
    );
    // And set its mtime
    filetime::set_file_mtime(full_path, now)?;
}

/// Check for a file
///
/// # `then file {filename} exists`
///
/// This simple step will succeed if the given filename exists in some sense.
#[step]
pub fn file_exists(context: &Datadir, filename: &Path) {
    let full_path = context.canonicalise_filename(filename)?;
    match fs::metadata(full_path) {
        Ok(_) => (),
        Err(e) => {
            if matches!(e.kind(), io::ErrorKind::NotFound) {
                throw!(format!("file '{}' was not found", filename.display()))
            } else {
                throw!(e);
            }
        }
    }
}

/// Check for absence of a file
///
/// # `then file {filename} does not exist`
///
/// This simple step will succeed if the given filename does not exist in any sense.
#[step]
pub fn file_does_not_exist(context: &Datadir, filename: &Path) {
    let full_path = context.canonicalise_filename(filename)?;
    match fs::metadata(full_path) {
        Ok(_) => {
            throw!(format!(
                "file '{}' was unexpectedly found",
                filename.display()
            ))
        }
        Err(e) => {
            if !matches!(e.kind(), io::ErrorKind::NotFound) {
                throw!(e);
            }
        }
    }
}

/// Check if a set of files are the only files in the datadir
///
/// # `then only files (?P<filenames>.+) exist`
///
/// This step iterates the data directory and checks that **only** the named files exist.
///
/// Note: `filenames` is whitespace-separated, though any commas are removed as well.
/// As such you cannot use this to test for filenames which contain commas.
#[step]
pub fn only_these_exist(context: &Datadir, filenames: &str) {
    let filenames: HashSet<OsString> = filenames
        .replace(',', "")
        .split_ascii_whitespace()
        .map(|s| s.into())
        .collect();
    let fnames: HashSet<OsString> = fs::read_dir(context.base_path())?
        .map(|entry| entry.map(|entry| entry.file_name()))
        .collect::<Result<_, _>>()?;
    assert_eq!(filenames, fnames);
}

/// Check if a file contains a given sequence of characters
///
/// # `then file (?P<filename>\S+) contains "(?P<data>.*)"`
///
/// This will load the content of the named file and ensure it contains the given string.
/// Note: this assumes everything is utf-8 encoded.  If not, things will fail.
#[step]
pub fn file_contains(context: &Datadir, filename: &Path, data: &str) {
    let full_path = context.canonicalise_filename(filename)?;
    let body = fs::read_to_string(full_path)?;
    if !body.contains(data) {
        println!("file {} contains:\n{}", filename.display(), body);
        throw!("expected file content not found");
    }
}

/// Check if a file lacks a given sequence of characters
///
/// # `then file (?P<filename>\S+) does not contain "(?P<data>.*)"`
///
/// This will load the content of the named file and ensure it lacks the given string.
/// Note: this assumes everything is utf-8 encoded.  If not, things will fail.
#[step]
pub fn file_doesnt_contain(context: &Datadir, filename: &Path, data: &str) {
    let full_path = context.canonicalise_filename(filename)?;
    let body = fs::read_to_string(full_path)?;
    if body.contains(data) {
        println!("file {} contains:\n{}", filename.display(), body);
        throw!("unexpected file content found");
    }
}

/// Check if a file's content matches the given regular expression
///
/// # `then file (?P<filename>\S+) matches regex /(?P<regex>.*)/`
///
/// This will load the content of th enamed file and ensure it contains data which
/// matches the given regular expression.  This step will fail if the file is not utf-8
/// encoded, or if the regex fails to compile
#[step]
pub fn file_matches_regex(context: &Datadir, filename: &Path, regex: &str) {
    let full_path = context.canonicalise_filename(filename)?;
    let regex = Regex::new(regex)?;
    let body = fs::read_to_string(full_path)?;
    if !regex.is_match(&body) {
        println!("file {} contains:\n{}", filename.display(), body);
        throw!("file content does not match given regex");
    }
}

/// Check if two files match
///
/// # `then files {filename1} and {filename2} match`
///
/// This loads the content of the given two files as **bytes** and checks they mach.
#[step]
pub fn file_match(context: &Datadir, filename1: &Path, filename2: &Path) {
    let full_path1 = context.canonicalise_filename(filename1)?;
    let full_path2 = context.canonicalise_filename(filename2)?;
    let body1 = fs::read(full_path1)?;
    let body2 = fs::read(full_path2)?;
    if body1 != body2 {
        println!(
            "file {} contains:\n{}",
            filename1.display(),
            String::from_utf8_lossy(&body1)
        );
        println!(
            "file {} contains:\n{}",
            filename2.display(),
            String::from_utf8_lossy(&body2)
        );
        throw!("file contents do not match each other");
    }
}

/// Check if a given file's metadata matches our memory of it
///
/// # `then file {filename} has same metadata as before`
///
/// This confirms that the metadata we remembered for the given filename
/// matches.  Specifically this checks:
///
/// * Are the permissions the same
/// * Are the modification times the same
/// * Is the file's length the same
/// * Is the file's type (file/dir) the same
#[step]
#[context(Datadir)]
#[context(Files)]
pub fn has_remembered_metadata(context: &ScenarioContext, filename: &Path) {
    let full_path = context.with(
        |context: &Datadir| context.canonicalise_filename(filename),
        false,
    )?;
    let metadata = fs::metadata(full_path)?;
    if let Some(remembered) = context.with(
        |context: &Files| Ok(context.metadata.get(filename).cloned()),
        false,
    )? {
        if metadata.permissions() != remembered.permissions()
            || metadata.modified()? != remembered.modified()?
            || metadata.len() != remembered.len()
            || metadata.is_file() != remembered.is_file()
        {
            throw!(format!(
                "metadata change detected for {}",
                filename.display()
            ));
        }
    } else {
        throw!(format!("no remembered metadata for {}", filename.display()));
    }
}

/// Check that a given file's metadata has changed since we remembered it
///
/// # `then file {filename} has different metadata from before`
///
/// This confirms that the metadata we remembered for the given filename
/// does not matche.  Specifically this checks:
///
/// * Are the permissions the same
/// * Are the modification times the same
/// * Is the file's length the same
/// * Is the file's type (file/dir) the same
#[step]
#[context(Datadir)]
#[context(Files)]
pub fn has_different_metadata(context: &ScenarioContext, filename: &Path) {
    let full_path = context.with(
        |context: &Datadir| context.canonicalise_filename(filename),
        false,
    )?;
    let metadata = fs::metadata(full_path)?;
    if let Some(remembered) = context.with(
        |context: &Files| Ok(context.metadata.get(filename).cloned()),
        false,
    )? {
        if metadata.permissions() == remembered.permissions()
            && metadata.modified()? == remembered.modified()?
            && metadata.len() == remembered.len()
            && metadata.is_file() == remembered.is_file()
        {
            throw!(format!(
                "metadata change not detected for {}",
                filename.display()
            ));
        }
    } else {
        throw!(format!("no remembered metadata for {}", filename.display()));
    }
}

/// Check if the given file has been modified "recently"
///
/// # `then file {filename} has a very recent modification time`
///
/// Specifically this checks that the given file has been modified in the past 5 seconds.
#[step]
pub fn mtime_is_recent(context: &Datadir, filename: &Path) {
    let full_path = context.canonicalise_filename(filename)?;
    let metadata = fs::metadata(full_path)?;
    let mtime = metadata.modified()?;
    let diff = SystemTime::now().duration_since(mtime)?;
    if diff > (Duration::from_secs(5)) {
        throw!(format!("{} is older than 5 seconds", filename.display()));
    }
}

/// Check if the given file is very old
///
/// # `then file {filename} has a very old modification time`
///
/// Specifically this checks that the file was modified at least 39 years ago.
#[step]
pub fn mtime_is_ancient(context: &Datadir, filename: &Path) {
    let full_path = context.canonicalise_filename(filename)?;
    let metadata = fs::metadata(full_path)?;
    let mtime = metadata.modified()?;
    let diff = SystemTime::now().duration_since(mtime)?;
    if diff < (Duration::from_secs(39 * 365 * 24 * 3600)) {
        throw!(format!("{} is younger than 39 years", filename.display()));
    }
}

/// Make a directory
///
/// # `given a directory {path}`
///
/// This is the equivalent of `mkdir -p` within the data directory for the scenario.
#[step]
pub fn make_directory(context: &Datadir, path: &Path) {
    context.create_dir_all(path)?;
}

/// Remove a directory
///
/// # `when I remove directory {path}`
///
/// This is the equivalent of `rm -rf` within the data directory for the scenario.
#[step]
pub fn remove_directory(context: &Datadir, path: &Path) {
    let full_path = context.canonicalise_filename(path)?;
    remove_dir_all::remove_dir_all(full_path)?;
}

/// Check that a directory exists
///
/// # `then directory {path} exists`
///
/// This ensures that the given path exists in the data directory for the scenario and
/// that it is a directory itself.
#[step]
pub fn path_exists(context: &Datadir, path: &Path) {
    let full_path = context.canonicalise_filename(path)?;
    if !fs::metadata(&full_path)?.is_dir() {
        throw!(format!(
            "{} exists but is not a directory",
            full_path.display()
        ))
    }
}

/// Check that a directory does not exist
///
/// # `then directory {path} does not exist`
///
/// This ensures that the given path does not exist in the data directory.  If it exists
/// and is not a directory, then this will also fail.
#[step]
pub fn path_does_not_exist(context: &Datadir, path: &Path) {
    let full_path = context.canonicalise_filename(path)?;
    match fs::metadata(&full_path) {
        Ok(_) => throw!(format!("{} exists", full_path.display())),
        Err(e) => {
            if !matches!(e.kind(), io::ErrorKind::NotFound) {
                throw!(e);
            }
        }
    };
}

/// Check that a directory exists and is empty
///
/// # `then directory {path} is empty`
///
/// This checks that the given path inside the data directory exists and is an
/// empty directory itself.
#[step]
pub fn path_is_empty(context: &Datadir, path: &Path) {
    let full_path = context.canonicalise_filename(path)?;
    let mut iter = fs::read_dir(&full_path)?;
    match iter.next() {
        None => {}
        Some(Ok(_)) => throw!(format!("{} is not empty", full_path.display())),
        Some(Err(e)) => throw!(e),
    }
}

/// Check that a directory exists and is not empty
///
/// # `then directory {path} is not empty`
///
/// This checks that the given path inside the data directory exists and is a
/// directory itself.  The step also asserts that the given directory contains at least
/// one entry.
#[step]
pub fn path_is_not_empty(context: &Datadir, path: &Path) {
    let full_path = context.canonicalise_filename(path)?;
    let mut iter = fs::read_dir(&full_path)?;
    match iter.next() {
        None => throw!(format!("{} is empty", full_path.display())),
        Some(Ok(_)) => {}
        Some(Err(e)) => throw!(e),
    }
}
