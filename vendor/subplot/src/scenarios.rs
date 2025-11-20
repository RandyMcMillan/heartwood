use crate::{html::Location, ScenarioStep};
use serde::{Deserialize, Serialize};

/// An acceptance test scenario.
///
/// A scenario consists of a title, by which it can be identified, and
/// a sequence of steps. The Scenario struct assumes the steps are
/// valid and make sense; the struct does not try to validate the
/// sequence.
#[derive(Debug, Serialize, Deserialize)]
pub struct Scenario {
    title: String,
    origin: Location,
    steps: Vec<ScenarioStep>,
}

impl Scenario {
    /// Construct a new scenario.
    ///
    /// The new scenario will have a title, but no steps.
    pub fn new(title: &str, origin: Location) -> Scenario {
        Scenario {
            title: title.to_string(),
            origin,
            steps: vec![],
        }
    }

    /// Return the title of a scenario.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Does the scenario have steps?
    pub fn has_steps(&self) -> bool {
        !self.steps.is_empty()
    }

    /// Return slice with all the steps.
    pub fn steps(&self) -> &[ScenarioStep] {
        &self.steps
    }

    /// Add a step to a scenario.
    pub fn add(&mut self, step: &ScenarioStep) {
        self.steps.push(step.clone());
    }

    pub(crate) fn origin(&self) -> &Location {
        &self.origin
    }
}

#[cfg(test)]
mod test {
    use super::Scenario;
    use crate::html::Location;
    use crate::ScenarioStep;
    use crate::StepKind;

    #[test]
    fn has_title() {
        let scen = Scenario::new("title", Location::Unknown);
        assert_eq!(scen.title(), "title");
    }

    #[test]
    fn has_no_steps_initially() {
        let scen = Scenario::new("title", Location::Unknown);
        assert_eq!(scen.steps().len(), 0);
    }

    #[test]
    fn adds_step() {
        let mut scen = Scenario::new("title", Location::Unknown);
        let step = ScenarioStep::new(StepKind::Given, "and", "foo", Location::Unknown);
        scen.add(&step);
        assert_eq!(scen.steps(), &[step]);
    }
}
