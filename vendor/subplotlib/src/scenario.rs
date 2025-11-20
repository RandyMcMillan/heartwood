//! Scenarios
//!
//! In general you will not need to interact with the [`Scenario`] type directly.
//! Instead instances of it are constructed in the generated test functions and
//! will be run automatically.

use core::fmt;
use std::fmt::Debug;
use std::{cell::RefCell, marker::PhantomData, sync::Mutex};

use state::Container;

use crate::step::ScenarioStep;
use crate::types::{StepError, StepResult};

/// A context element is anything which can be used as a scenario step context.
///
/// Contexts get called whenever the scenario steps occur so that they can do
/// prep, cleanup, etc.  It's important for authors of context element types to
/// be aware that they won't always be called on scenario start and they will
/// not be caught up the first time they are invoked for a step, simply expected
/// to get on with life from their first use.
pub trait ContextElement: Debug + Default + Send + 'static {
    /// A new context element was created.
    ///
    /// In order to permit elements which for example work on disk, this
    /// function will be invoked with the scenario's context to permit the
    /// context to register other contexts it might need, or to permit the
    /// creation of suitably named temporary directories, logging, etc.
    ///
    /// The scenario's title is available via [`scenario_context.title()`][title]
    ///
    /// [title]: [ScenarioContext::title]
    #[allow(unused_variables)]
    fn created(&mut self, scenario: &Scenario) {
        // Nothing by default
    }

    /// Scenario starts
    ///
    /// When a scenario starts, this function is called to permit setup.
    ///
    /// If this returns an error, scenario setup is stopped and `scenario_stops`
    /// will be called for anything which succeeded at startup.
    fn scenario_starts(&mut self) -> StepResult {
        Ok(())
    }

    /// Scenario stops
    ///
    /// When a scenario finishes, this function is called to permit teardown.
    ///
    /// If this returns an error, and the scenario would otherwise have passed,
    /// then the error will be used.  The first encountered error in stopping
    /// a scenario will be used, rather than the last.  All contexts which
    /// succeeded at starting will be stopped.
    fn scenario_stops(&mut self) -> StepResult {
        Ok(())
    }

    /// Entry to a step function
    ///
    /// In order to permit elements which for example work on disk, this
    /// function will be invoked with the step's name to permit the creation of
    /// suitably named temporary directories, logging, etc.
    ///
    /// The default implementation of this does nothing.
    ///
    /// Calls to this function *will* be paired with calls to the step exit
    /// function providing nothing panics or calls exit without unwinding.
    ///
    /// If you wish to be resilient to step functions panicing then you will
    /// need to be careful to cope with a new step being entered without a
    /// previous step exiting.  Particularly if you're handing during cleanup
    /// of a failed scenario.
    ///
    /// If this returns an error then the step function is not run, nor is the
    /// corresponding `exit_step()` called.
    #[allow(unused_variables)]
    fn step_starts(&mut self, step_title: &str) -> StepResult {
        Ok(())
    }

    /// Exit from a step function
    ///
    /// See [the `step_starts` function][ContextElement::step_starts] for most
    /// details of this.
    ///
    /// Any error returned from this will be masked if the step function itself
    /// returned an error.  However if the step function succeeded then this
    /// function's error will make it out.
    #[allow(unused_variables)]
    fn step_stops(&mut self) -> StepResult {
        Ok(())
    }
}

/// A scenario context wrapper for a particular context type
struct ScenarioContextItem<C>(Mutex<C>);

/// A type hook used purely in order to be able to look up contexts in the
/// container in order to be able to iterate them during scenario execution
struct ScenarioContextHook<C>(PhantomData<C>);

impl<C> ScenarioContextHook<C>
where
    C: ContextElement,
{
    fn new() -> Self {
        Self(PhantomData)
    }
}

/// A trait used to permit the holding of multiple hooks in one vector
trait ScenarioContextHookKind {
    /// Start scenario
    fn scenario_starts(&self, contexts: &ScenarioContext) -> StepResult;

    /// Stop scenario
    fn scenario_stops(&self, contexts: &ScenarioContext) -> StepResult;

    /// Enter a step
    fn step_starts(&self, contexts: &ScenarioContext, step_name: &str) -> StepResult;

    /// Leave a step
    fn step_stops(&self, contexts: &ScenarioContext) -> StepResult;

    /// Produce your debug output
    fn debug(&self, contexts: &ScenarioContext, dc: &mut DebuggedContext, alternate: bool);
}

impl<C> ScenarioContextHookKind for ScenarioContextHook<C>
where
    C: ContextElement,
{
    fn scenario_starts(&self, contexts: &ScenarioContext) -> StepResult {
        contexts.with_mut(|c: &mut C| c.scenario_starts(), false)
    }

    fn scenario_stops(&self, contexts: &ScenarioContext) -> StepResult {
        contexts.with_mut(|c: &mut C| c.scenario_stops(), true)
    }

    fn step_starts(&self, contexts: &ScenarioContext, step_name: &str) -> StepResult {
        contexts.with_mut(|c: &mut C| c.step_starts(step_name), false)
    }

    fn step_stops(&self, contexts: &ScenarioContext) -> StepResult {
        contexts.with_mut(|c: &mut C| c.step_stops(), true)
    }

    fn debug(&self, contexts: &ScenarioContext, dc: &mut DebuggedContext, alternate: bool) {
        contexts.with_generic(|c: &C| dc.add(c, alternate));
    }
}

/// A container for all scenario contexts
///
/// This container allows the running of code within a given scenario context.
pub struct ScenarioContext {
    title: String,
    location: &'static str,
    inner: Container![],
    hooks: RefCell<Vec<Box<dyn ScenarioContextHookKind>>>,
}

#[derive(Default)]
struct DebuggedContext {
    body: Vec<String>,
}

impl DebuggedContext {
    fn add<C>(&mut self, obj: &C, alternate: bool)
    where
        C: Debug,
    {
        let body = if alternate {
            format!("{obj:#?}")
        } else {
            format!("{obj:?}")
        };
        self.body.push(body);
    }
}

struct DebugContextString<'a>(&'a str);

impl<'a> Debug for DebugContextString<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl Debug for DebuggedContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(self.body.iter().map(|s| DebugContextString(s)))
            .finish()
    }
}

impl Debug for ScenarioContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut contexts = DebuggedContext::default();
        for hook in self.hooks.borrow().iter() {
            hook.debug(self, &mut contexts, f.alternate());
        }
        f.debug_struct("ScenarioContext")
            .field("title", &self.title)
            .field("contexts", &contexts)
            .finish()
    }
}

impl ScenarioContext {
    fn new(title: &str, location: &'static str) -> Self {
        Self {
            title: title.to_string(),
            location,
            inner: <Container![]>::new(),
            hooks: RefCell::new(Vec::new()),
        }
    }

    /// The title for this scenario
    fn title(&self) -> &str {
        &self.title
    }

    /// Ensure a context is registered
    pub(crate) fn register_context_type<C>(&self) -> bool
    where
        C: ContextElement,
    {
        let sci: Option<&ScenarioContextItem<C>> = self.inner.try_get();
        if sci.is_none() {
            let ctx = ScenarioContextItem(Mutex::new(C::default()));
            self.inner.set(ctx);
            self.hooks
                .borrow_mut()
                .push(Box::new(ScenarioContextHook::<C>::new()));
            true
        } else {
            false
        }
    }

    fn with_generic<C, F>(&self, func: F)
    where
        F: FnOnce(&C),
        C: ContextElement,
    {
        let sci: &ScenarioContextItem<C> = self
            .inner
            .try_get()
            .expect("Scenario Context item not initialised");
        let lock = match sci.0.lock() {
            Ok(lock) => lock,
            Err(pe) => pe.into_inner(),
        };
        func(&lock)
    }

    /// With the extracted immutable context, run the function f.
    pub fn with<C, F, R>(&self, func: F, defuse_poison: bool) -> Result<R, StepError>
    where
        F: FnOnce(&C) -> Result<R, StepError>,
        C: ContextElement,
    {
        self.with_mut(|c: &mut C| func(&*c), defuse_poison)
    }

    /// With the extracted mutable context, run the function f.
    pub fn with_mut<C, F, R>(&self, func: F, defuse_poison: bool) -> Result<R, StepError>
    where
        F: FnOnce(&mut C) -> Result<R, StepError>,
        C: ContextElement,
    {
        let sci: &ScenarioContextItem<C> = self
            .inner
            .try_get()
            .ok_or("required context type not registered with scenario")?;
        let mut lock = match sci.0.lock() {
            Ok(lock) => lock,
            Err(pe) => {
                if defuse_poison {
                    pe.into_inner()
                } else {
                    return Err("context poisoned by panic".into());
                }
            }
        };
        func(&mut lock)
    }
}

/// The embodiment of a subplot scenario
///
/// Scenario objects are built up by the generated test functions and then run
/// to completion.  In rare cases they may be built up and cached for reuse
/// for example if a scenario is a subroutine.
///
/// Scenarios are built from steps in sequence, and then can be run.
///
/// ```
/// # use subplotlib::prelude::*;
///
/// let mut scenario = Scenario::new("example scenario", "unknown");
///
/// let run_step = subplotlib::steplibrary::runcmd::run::Builder::default()
///     .argv0("true")
///     .args("")
///     .build("when I run true".to_string(), "unknown");
/// scenario.add_step(run_step, None);
///
/// ```
pub struct Scenario {
    contexts: ScenarioContext,
    steps: Vec<(ScenarioStep, Option<ScenarioStep>)>,
}

impl Scenario {
    /// Create a new scenario with the given title
    pub fn new(title: &str, location: &'static str) -> Self {
        Self {
            contexts: ScenarioContext::new(title, location),
            steps: Vec::new(),
        }
    }

    /// Retrieve the scenario title
    pub fn title(&self) -> &str {
        self.contexts.title()
    }

    /// Add a scenario step, with optional cleanup step function.
    pub fn add_step(&mut self, step: ScenarioStep, cleanup: Option<ScenarioStep>) {
        step.register_contexts(self);
        if let Some(s) = cleanup.as_ref() {
            s.register_contexts(self)
        }
        self.steps.push((step, cleanup));
    }

    /// Register a type with the scenario contexts
    pub fn register_context_type<C>(&self)
    where
        C: ContextElement,
    {
        if self.contexts.register_context_type::<C>() {
            self.contexts
                .with_mut(
                    |c: &mut C| {
                        c.created(self);
                        Ok(())
                    },
                    false,
                )
                .unwrap();
        }
    }

    /// Run the scenario to completion.
    ///
    /// Running the scenario to completion requires running each step in turn.
    /// This will return the first encountered error, or unit if the scenario
    /// runs cleanly.
    ///
    /// # Panics
    ///
    /// If any of the cleanup functions error, this will immediately panic.
    ///
    pub fn run(self) -> Result<(), StepError> {
        // Firstly, we start all the contexts
        let mut ret = Ok(());
        let mut highest_start = None;
        println!(
            "{}: scenario: {}",
            self.contexts.location,
            self.contexts.title()
        );
        for (i, hook) in self.contexts.hooks.borrow().iter().enumerate() {
            let res = hook.scenario_starts(&self.contexts);
            if res.is_err() {
                ret = res;
                break;
            }
            highest_start = Some(i);
        }
        if ret.is_err() {
            println!("*** Context hooks returned failure",);
        }
        if ret.is_ok() {
            let mut highest = None;
            for (i, step) in self.steps.iter().map(|(step, _)| step).enumerate() {
                println!("{}:   step: {}", step.location(), step.step_text());
                let mut highest_prep = None;
                for (i, prep) in self.contexts.hooks.borrow().iter().enumerate() {
                    let res = prep.step_starts(&self.contexts, step.step_text());
                    if res.is_err() {
                        ret = res;
                        break;
                    }
                    highest_prep = Some(i);
                }
                if ret.is_err() {
                    println!("*** Context hooks returned failure",);
                }
                if ret.is_ok() {
                    let res = step.call(&self.contexts, false);
                    if res.is_err() {
                        ret = res;
                        break;
                    }
                    highest = Some(i);
                }
                if let Some(n) = highest_prep {
                    for hookn in (0..=n).rev() {
                        let res = self.contexts.hooks.borrow()[hookn].step_stops(&self.contexts);
                        ret = ret.and(res)
                    }
                }
            }
            if let Some(n) = highest {
                for stepn in (0..=n).rev() {
                    if let (_, Some(cleanup)) = &self.steps[stepn] {
                        println!("  cleanup: {}", cleanup.step_text());
                        let res = cleanup.call(&self.contexts, true);
                        if res.is_err() {
                            println!("*** Cleanup returned failure",);
                        }
                        ret = ret.and(res);
                    }
                }
            }
        }

        if let Some(n) = highest_start {
            for hookn in (0..=n).rev() {
                let res = self.contexts.hooks.borrow()[hookn].scenario_stops(&self.contexts);
                ret = ret.and(res);
            }
        }
        println!("  return: {}", if ret.is_ok() { "OK" } else { "Failure" });
        ret
    }
}
