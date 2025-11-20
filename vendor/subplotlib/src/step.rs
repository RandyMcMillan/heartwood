//! Scenario steps
//!
//! In general you will not need to interact with these types, they're simply
//! here to provide wrappers to make the scenarios easier to work with.
//!

use std::any::Any;
use std::panic::{catch_unwind, AssertUnwindSafe};

use crate::scenario::{Scenario, ScenarioContext};
use crate::types::StepResult;

type StepFunc = dyn Fn(&ScenarioContext, bool) -> StepResult;

/// A ScenarioStep is one step in a scenario.
///
/// In essence, a scenario step is a named closure.  Its name can be used when
/// reporting an error encountered in running a scenario.
///
/// Scenario steps are typically constructed from step builders, rather than
/// directly.  This permits the step builder to correctly register context types
/// etc.
///
/// ```
/// # use subplotlib::prelude::*;
///
/// let step = ScenarioStep::new(
///     "when everything works".to_string(), |ctx, ok| Ok(()), |scen| (), "unknown"
/// );
/// ```
pub struct ScenarioStep {
    step_text: String,
    location: &'static str,
    func: Box<StepFunc>,
    reg: Box<dyn Fn(&Scenario)>,
}

impl ScenarioStep {
    /// Create a new scenario step taking the scenario context
    ///
    /// This is used to construct a scenario step from a function which
    /// takes the scenario context container.  This will generally be
    /// called from the generated build method for the step.
    pub fn new<F, R>(step_text: String, func: F, reg: R, location: &'static str) -> Self
    where
        F: Fn(&ScenarioContext, bool) -> StepResult + 'static,
        R: Fn(&Scenario) + 'static,
    {
        Self {
            step_text,
            location,
            func: Box::new(func),
            reg: Box::new(reg),
        }
    }

    /// Attempt to render a message.
    /// If something panics with a type other than a static string or
    /// a formatted string then we won't be able to render it sadly.
    fn render_panic(location: &str, name: &str, err: Box<dyn Any + Send>) -> String {
        if let Some(msg) = err.downcast_ref::<&str>() {
            format!("{location}: step {name} panic'd: {msg}")
        } else if let Some(msg) = err.downcast_ref::<String>() {
            format!("{location}: step {name} panic'd: {msg}")
        } else {
            format!("{location}: step {name} panic'd")
        }
    }

    /// Call the step function
    ///
    /// This simply calls the encased step function
    pub fn call(&self, context: &ScenarioContext, defuse_poison: bool) -> StepResult {
        // Note, panic here will be absorbed and so there's a risk that
        // subsequent step calls may not be sound.  There's not a lot we can
        // do to ensure things are good except try.
        let func = AssertUnwindSafe(|| (*self.func)(context, defuse_poison));
        catch_unwind(func).map_err(|e| Self::render_panic(self.location(), self.step_text(), e))?
    }

    /// Return the full text of this step
    pub fn step_text(&self) -> &str {
        &self.step_text
    }

    /// Register any context types needed by this step
    pub(crate) fn register_contexts(&self, scenario: &Scenario) {
        (*self.reg)(scenario);
    }

    pub(crate) fn location(&self) -> &'static str {
        self.location
    }
}
