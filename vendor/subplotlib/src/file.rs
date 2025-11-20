//! Subplot data files
//!
//! Subplot can embed data files into test suites.  This module provides
//! the representation of the files in a way designed to be cheap to clone
//! so that they can be passed around "by value".

use std::path::{Path, PathBuf};
use std::sync::Arc;

use base64::prelude::{Engine as _, BASE64_STANDARD};

/// An embedded data file.
///
/// Embedded data files have names and content.  The subplot template will generate
/// a `lazy_static` containing all the data files embedded into the suite.  Then
/// generated test functions will extract data files
///
/// If you are using them in your test steps you should take them by value:
///
/// ```rust
/// # use subplotlib::prelude::*;
/// # #[derive(Debug, Default)]
/// # struct Context {}
/// # impl ContextElement for Context {}
/// #[step]
/// fn step_using_a_file(context: &mut Context, somefile: SubplotDataFile) {
///     // context.stash_file(somefile);
/// }
///
/// # fn main() {}
/// ```
///
/// For the generated test to correctly recognise how to pass a file in, you
/// **must** mark the argument as a file in your binding:
///
/// ```yaml
/// - when: using {somefile} as a input
///   function: step_using_a_file
///   types:
///     somefile: file
/// ```
#[derive(Debug)]
pub struct SubplotDataFile {
    name: Arc<Path>,
    data: Arc<[u8]>,
}

impl Clone for SubplotDataFile {
    fn clone(&self) -> Self {
        Self {
            name: Arc::clone(&self.name),
            data: Arc::clone(&self.data),
        }
    }
}

impl Default for SubplotDataFile {
    fn default() -> Self {
        Self {
            name: PathBuf::from("").into(),
            data: Vec::new().into(),
        }
    }
}

impl SubplotDataFile {
    /// Construct a new data file object
    ///
    /// Typically this will only be called from the generated test suite.
    /// The passed in name and data must be base64 encoded strings and each will
    /// be interpreted independently.  The name will be treated as a [`PathBuf`]
    /// and the data will be stored as a slice of bytes.
    ///
    /// Neither will be interpreted as utf8.
    ///
    /// ```
    /// # use subplotlib::prelude::*;
    ///
    /// let data_file = SubplotDataFile::new("aGVsbG8=", "d29ybGQ=");
    /// ```
    ///
    /// # Panics
    ///
    /// This will panic if the passed in strings are not correctly base64 encoded.
    pub fn new(name: &str, data: &str) -> Self {
        let name = BASE64_STANDARD
            .decode(name)
            .expect("Subplot generated bad base64?");
        let name = String::from_utf8_lossy(&name);
        let name: PathBuf = name.as_ref().into();
        let name = name.into();
        let data = BASE64_STANDARD
            .decode(data)
            .expect("Subplot generated bad base64?")
            .into();
        Self { name, data }
    }

    /// Retrieve the filename
    ///
    /// File names are returned as a borrow of a [`Path`] since they could be
    /// arbitrarily constructed, though typically they'll have come from markdown
    /// source and so likely they will be utf-8 compatible.
    ///
    /// ```
    /// # use subplotlib::prelude::*;
    /// let data_file = SubplotDataFile::new("aGVsbG8=", "d29ybGQ=");
    /// assert_eq!(data_file.name().display().to_string(), "hello");
    /// ```
    pub fn name(&self) -> &Path {
        &self.name
    }

    /// Retrieve the data
    ///
    /// The data of a file is always returned as a slice of bytes.  This is because
    /// files could be arbitrary data, though again, they typically will have been
    /// sourced from a Subplot document and so be utf-8 compatible.
    ///
    /// ```
    /// # use subplotlib::prelude::*;
    ///
    /// let data_file = SubplotDataFile::new("aGVsbG8=", "d29ybGQ=");
    /// assert_eq!(data_file.data(), b"world");
    /// ```
    pub fn data(&self) -> &[u8] {
        &self.data
    }
}
