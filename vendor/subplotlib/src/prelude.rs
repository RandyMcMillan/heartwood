//! The subplotlib prelude.
//!
//! This prelude is automatically imported into all generated subplot test suites
//! using the `rust` template.  Effectively they get
//!
//! ```rust
//! use subplotlib::prelude::*;
//! ```
//!
//! inserted into them at the top of the file.
//!
//! You should familiarise yourself with the context-related types if you
//! are writing your own contexts, or writing step functions which use
//! the contexts provided by the [step library][crate::steplibrary] itself.
//!
//! The primary thing you will need to learn, as a step function author, is
//! the [`#[step]`][macro@step] attribute macro, and it interacts with contexts.

// Re-export culpa's macros so that step functions can use them
pub use culpa::throw;
/// Indicate what type a function throws
///
/// This attribute macro comes from the [`culpa`] crate and is used
/// to indicate that a function "throws" a particular kind of error.
///
/// ```rust
/// # use std::io;
/// # use culpa::throws;
/// #[throws(io::Error)]
/// fn create_thingy() {
///     // something which might cause an io::Error
/// }
/// # fn main() {}
/// ```
///
/// It transforms a function such that the above function would be compiled
/// effectively as:
///
/// ```rust
/// # use std::io;
/// fn create_thingy() -> Result<(), io::Error> {
///     // something which might cause an io::Error
///     Ok(())
/// }
/// ```
///
/// Return statements and the final expression of the function automatically
/// get wrappered with [`Ok`][std::result::Result::Ok].  You can use the
/// [`throw`][macro@throw] macro inside such a function to automatically return
/// an error.
///
// When https://github.com/rust-lang/rust/issues/83976 is resolved, the below
// can be added to this doc string.
// You can see more documentation on this in the [culpa crate docs][culpa::throws].
// Alternatively we can get rid of this entirely if and when
// https://github.com/rust-lang/rust/issues/81893 is fixed.
pub use culpa::throws;

// Re-export the lazy_static macro
#[doc(hidden)]
pub use lazy_static::lazy_static;

// Re-export subplotlib-derive's macros so that #[step] works
// We have to document it here because we're referring to things
// inside subplotlib.
/// Mark a function as a Subplot step function.
///
/// This attribute macro is used to indicate that a given function is
/// a step which can be bound to Subplot scenario statements.
///
/// In its simplest form, you use this thusly:
///
/// ```rust
/// # use subplotlib::prelude::*;
/// # #[derive(Debug, Default)] struct SomeContextType;
/// # impl ContextElement for SomeContextType {}
/// #[step]
/// fn my_step_function(context: &mut SomeContextType, arg1: &str, arg2: &str)
/// {
///     // Do something appropriate here.
/// }
/// # fn main() {}
/// ```
///
/// Step functions must be pretty basic.  For now at least they cannot be async,
/// and they cannot be generic, take variadic arguments, be unsafe, etc.
///
/// The first argument to the step function is considered the step's context.
/// Anything which implements
/// [`ContextElement`] can be used here. You
/// can take either a `&ContextType` or `&mut ContextType` as your context. It's
/// likely best to take the shared borrow if you're not altering the context at all.
///
/// If you require access to a number of context objects as part of your step
/// function implementation, then there is the high level container
/// [`ScenarioContext`].  You should take
/// this high level container as a shared borrow, and then within your step function
/// you can access other contexts as follows:
///
/// ```rust
/// # use subplotlib::prelude::*;
/// # #[derive(Debug, Default)] struct ContextA;
/// # #[derive(Debug, Default)] struct ContextB;
/// # impl ContextElement for ContextA {}
/// # impl ContextElement for ContextB {}
/// # impl ContextA { fn get_thingy(&self) -> Result<usize, StepError> { Ok(0) } }
/// # impl ContextB { fn do_mut_thing(&mut self, _n: usize, _arg: &str) -> Result<(), StepError> { Ok(()) } }
/// #[step]
/// #[context(ContextA)]
/// #[context(ContextB)]
/// fn my_step(context: &ScenarioContext, arg: &str) {
///     let thingy = context.with(|ctx: &ContextA| ctx.get_thingy(), false )?;
///     context.with_mut(|ctx: &mut ContextB| ctx.do_mut_thing(thingy, arg), false)?;
/// }
/// # fn main() {}
/// ```
///
/// Importantly here there is the `#[context(SomeContext)]` attribute to tell
/// the system that you use that context in your step function (without this,
/// the relevant context may not be initialised for the scenario).  The mechanism
/// to then access contexts from the step function is the
/// [`ScenarioContext::with`] and
/// [`ScenarioContext::with_mut`]
/// functions which allow shared, or mutable, access to scenario contexts respectively.
///
/// These functions take two arguments, the first is a closure which will be run
/// with access to the given context and whose return value will be ultimately
/// returned by the call to the function.  The second is whether or not to defuse
/// the poison on the context mutex.  In normal steps this should be `false` since
/// you want a step to fail if the context has been poisoned.  However, in cleanup
/// related step functions you probably want to defuse the poison and be careful in
/// how you then use the contexts so that you can clean up effectively.
pub use subplotlib_derive::step;

// Re-export the step result types
#[doc(inline)]
pub use crate::types::StepError;
#[doc(inline)]
pub use crate::types::StepResult;

// And the step itself
#[doc(inline)]
pub use crate::step::ScenarioStep;

// Data files
#[doc(inline)]
pub use crate::file::SubplotDataFile;

// Finally a scenario itself
#[doc(inline)]
pub use crate::scenario::ContextElement;
#[doc(inline)]
pub use crate::scenario::Scenario;
#[doc(inline)]
pub use crate::scenario::ScenarioContext;

// Utility functions
#[doc(inline)]
pub use crate::utils::base64_decode;
