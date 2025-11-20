//! Subplot test utilities
//!
//! This crate is meant to be used by test suites generated using the [Subplot][]
//! code generator using the `rust` template.  You should generally not use this
//! crate directly, instead test suites generated using the `rust` template will
//! `use subplotlib::prelude::*;` automatically.
//!
//! If you want to explore things, then we suggest you start from the [`prelude`][mod@prelude]
//! module since it exposes everything you'd normally find in a test suite.
//!
//! [Subplot]: https://subplot.liw.fi/

pub mod file;
pub mod prelude;
pub mod scenario;
pub mod step;
pub mod steplibrary;
pub mod types;
pub mod utils;
