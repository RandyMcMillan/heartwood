//! Subplot step library
//!
//! In the Python and shell runners, there are some libraries to help write
//! subplots which provide things like assistance with stored files, checking of
//! file content, running of subprocesses, managing of directories of data, etc.
//!
//! These submodules contain steps to that effect.  In order to use them you
//! will want to:
//!
//! ```
//! use subplotlib::steplibrary::datadir::*;
//! ```
//!
//! or similar.
//!
//! Then you can simply use the bindings for the library in question from
//! requisite yaml files.

pub mod datadir;
pub mod files;
pub mod runcmd;
