//! Fundamental types used throughout subplotlib

/// Generic error type which steps can return.
///
/// Step functions constructed using the [`step`][macro@subplotlib_derive::step]
/// macro will automatically return this as the error type.  It is pretty
/// generic meaning that almost every possible use of the `?` operator should
/// work.
pub type StepError = ::std::boxed::Box<dyn ::std::error::Error>;

/// Result type using [`StepError`].
///
/// This is useful for use in situations where you
/// might use [`#[throws(...)]`][macro@culpa::throws].  Step functions
/// generated using the [`step`][macro@subplotlib_derive::step] macro will
/// automatically set this as their return type by means of `#[throws(StepResult)]`.
pub type StepResult = ::std::result::Result<(), StepError>;
