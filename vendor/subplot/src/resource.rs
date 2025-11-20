//! Resources for subplot.
//!
//! This module encapsulates a mechanism for subplot to find resource files.
//! Resource files come from a number of locations, such as embedded into the
//! binary, from template paths given on the CLI, or from paths built into the
//! binary and present on disk.

use clap::Parser;
use std::io::{self, Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[allow(missing_docs)]
#[derive(Debug, Parser)]
// Options which relate to resource management
//
// To use this, include them *flat* in your options struct, and then after
// parsing, call the [ResourceOpts::handle()] function.
pub struct ResourceOpts {
    #[clap(
        long,
        number_of_values = 1,
        help = "Look for code templates and other resources in DIR",
        name = "DIR"
    )]
    resources: Vec<PathBuf>,
}

impl ResourceOpts {
    /// Handle any supplied resource related arguments
    pub fn handle<P: AsRef<Path>>(&self, doc_path: Option<P>) {
        for rpath in &self.resources {
            add_search_path(rpath);
        }
        if let Some(doc_path) = doc_path.as_ref() {
            add_search_path(doc_path);
        }
    }
}

use lazy_static::lazy_static;

lazy_static! {
    static ref SEARCH_PATHS: Mutex<Vec<PathBuf>> = {
        let ret = Vec::new();
        Mutex::new(ret)
    };
}

static EMBEDDED_FILES: &[(&str, &[u8])] = include!(concat!(env!("OUT_DIR"), "/embedded_files.rs"));

/// Retrieve the embedded file list
pub fn embedded_files() -> &'static [(&'static str, &'static [u8])] {
    EMBEDDED_FILES
}

/// Put a path at the back of the queue to search in
pub fn add_search_path<P: AsRef<Path>>(path: P) {
    SEARCH_PATHS
        .lock()
        .expect("Unable to lock SEARCH_PATHS")
        .push(path.as_ref().into());
}

/// Open a file for reading, honouring search paths established during
/// startup, and falling back to potentially embedded file content.
///
/// The search path sequence is:
///
/// First any path given to `--resources` in the order of the arguments
/// Second, it's relative to the input document.
/// Finally we check for an embedded file.
///
/// Then we repeat all the above, inserting 'common' as the template name, and
/// finally repeat the above inserting the real template name in the subpath
/// too.
fn open<P: AsRef<Path>>(subpath: P, template: Option<&str>) -> io::Result<Box<dyn Read>> {
    let subpath = subpath.as_ref();
    let plain = match internal_open(subpath) {
        Ok(r) => return Ok(r),
        Err(e) => e,
    };
    let commonpath = Path::new("common").join(subpath);
    let common = match internal_open(&commonpath) {
        Ok(r) => return Ok(r),
        Err(e) => e,
    };
    let templated = match template {
        Some(templ) => {
            let templpath = Path::new(templ).join(subpath);
            match internal_open(&templpath) {
                Ok(r) => return Ok(r),
                Err(e) => Some(e),
            }
        }
        None => None,
    };
    if plain.kind() != io::ErrorKind::NotFound {
        return Err(plain);
    }
    if common.kind() != io::ErrorKind::NotFound {
        return Err(common);
    }
    match templated {
        Some(e) => Err(e),
        None => Err(common),
    }
}

fn internal_open(subpath: &Path) -> io::Result<Box<dyn Read>> {
    let search_paths = SEARCH_PATHS.lock().expect("Unable to lock SEARCH_PATHS");
    let search_paths = search_paths.iter().map(|p| p.as_path());
    let search_paths = std::iter::empty().chain(search_paths);
    let mut ret = Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("Unable to find {} in resource paths", subpath.display()),
    ));
    for basepath in search_paths {
        let full_path = basepath.join(subpath);
        ret = std::fs::File::open(full_path);
        if ret.is_ok() {
            break;
        }
    }

    match ret {
        Ok(ret) => Ok(Box::new(ret)),
        Err(e) => {
            if let Some(data) = EMBEDDED_FILES.iter().find(|e| Path::new(e.0) == subpath) {
                Ok(Box::new(Cursor::new(data.1)))
            } else {
                Err(e)
            }
        }
    }
}

/// Read a file, honouring search paths established during startup, and
/// falling back to potentially embedded file content
pub fn read_as_string<P: AsRef<Path>>(subpath: P, template: Option<&str>) -> io::Result<String> {
    let mut f = open(subpath, template)?;
    let mut ret = String::with_capacity(8192);
    f.read_to_string(&mut ret)?;
    Ok(ret)
}
