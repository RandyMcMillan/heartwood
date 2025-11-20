use crate::html::Location;
use crate::Binding;
use crate::Scenario;
use crate::StepKind;
use crate::SubplotError;
use crate::{bindings::CaptureType, Bindings};

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A scenario that has all of its steps matched with steps using bindings.
#[derive(Debug, Serialize, Deserialize)]
pub struct MatchedScenario {
    title: String,
    origin: Location,
    steps: Vec<MatchedStep>,
}

impl MatchedScenario {
    /// Construct a new matched scenario
    pub fn new(
        template: &str,
        scen: &Scenario,
        bindings: &Bindings,
    ) -> Result<MatchedScenario, SubplotError> {
        let steps: Result<Vec<MatchedStep>, SubplotError> = scen
            .steps()
            .iter()
            .map(|step| bindings.find(template, step))
            .collect();
        Ok(MatchedScenario {
            title: scen.title().to_string(),
            origin: scen.origin().clone(),
            steps: steps?,
        })
    }

    /// Get the steps in this matched scenario
    pub fn steps(&self) -> &[MatchedStep] {
        &self.steps
    }

    /// Retrieve the scenario title
    pub fn title(&self) -> &str {
        &self.title
    }
}

/// A list of matched steps.
#[derive(Debug)]
pub struct MatchedSteps {
    matches: Vec<MatchedStep>,
}

impl MatchedSteps {
    /// Create new set of steps that match a scenario step.
    pub fn new(matches: Vec<MatchedStep>) -> Self {
        Self { matches }
    }
}

impl std::fmt::Display for MatchedSteps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        let matches: Vec<String> = self.matches.iter().map(|m| m.pattern()).collect();
        write!(f, "{}", matches.join("\n"))
    }
}

/// A matched binding and scenario step, with captured parts.
///
/// A MatchedStep is a sequence of partial steps, each representing
/// either a captured or a matching part of the text of the step.
#[derive(Debug, Serialize, Deserialize)]
pub struct MatchedStep {
    kind: StepKind,
    pattern: String,
    text: String,
    origin: Location,
    parts: Vec<PartialStep>,
    function: Option<String>,
    cleanup: Option<String>,
    types: HashMap<String, CaptureType>,
}

impl MatchedStep {
    /// Return a new empty match. Empty means it has no step parts.
    pub fn new(binding: &Binding, template: &str, origin: Location) -> MatchedStep {
        let bimpl = binding.step_impl(template);
        MatchedStep {
            kind: binding.kind(),
            pattern: binding.pattern().to_string(),
            text: "".to_string(),
            origin,
            parts: vec![],
            function: bimpl.clone().map(|b| b.function().to_owned()),
            cleanup: bimpl.and_then(|b| b.cleanup().map(String::from)),
            types: binding.types().map(|(s, c)| (s.to_string(), c)).collect(),
        }
    }

    /// The kind of step that has been matched.
    pub fn kind(&self) -> StepKind {
        self.kind
    }

    /// The name of the function to call for the step.
    pub fn function(&self) -> Option<&str> {
        self.function.as_deref()
    }

    /// Append a partial step to the match.
    pub fn append_part(&mut self, part: PartialStep) {
        self.parts.push(part);
        self.text = self.update_text();
    }

    /// Iterate over all partial steps in the match.
    pub fn parts(&self) -> impl Iterator<Item = &PartialStep> {
        self.parts.iter()
    }

    fn pattern(&self) -> String {
        self.pattern.to_string()
    }

    fn update_text(&self) -> String {
        let mut t = String::new();
        for part in self.parts() {
            t.push_str(part.as_text());
        }
        t
    }

    /// Return the step's text
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Return the typemap for the matched step
    pub fn types(&self) -> &HashMap<String, CaptureType> {
        &self.types
    }
}

/// Part of a scenario step, possibly captured by a pattern.
#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PartialStep {
    /// The text of a step part that isn't captured.
    UncapturedText(StepSnippet),

    /// A captured step part that is just text, and the regex capture
    /// name that corresponds to it.
    CapturedText {
        /// Name of the capture.
        name: String,
        /// Text of the capture.
        text: String,
        /// Type of capture.
        kind: CaptureType,
    },
}

impl PartialStep {
    /// Construct an uncaptured part of a scenario step.
    pub fn uncaptured(text: &str) -> PartialStep {
        PartialStep::UncapturedText(StepSnippet::new(text))
    }

    /// Construct a textual captured part of a scenario step.
    pub fn text(name: &str, text: &str, kind: CaptureType) -> PartialStep {
        PartialStep::CapturedText {
            name: name.to_string(),
            text: text.to_string(),
            kind,
        }
    }

    fn as_text(&self) -> &str {
        match self {
            PartialStep::UncapturedText(snippet) => snippet.text(),
            PartialStep::CapturedText { text, .. } => text,
        }
    }
}

#[cfg(test)]
mod test_partial_steps {
    use super::PartialStep;
    use crate::bindings::CaptureType;

    #[test]
    fn identical_uncaptured_texts_match() {
        let p1 = PartialStep::uncaptured("foo");
        let p2 = PartialStep::uncaptured("foo");
        assert_eq!(p1, p2);
    }

    #[test]
    fn different_uncaptured_texts_dont_match() {
        let p1 = PartialStep::uncaptured("foo");
        let p2 = PartialStep::uncaptured("bar");
        assert_ne!(p1, p2);
    }

    #[test]
    fn identical_captured_texts_match() {
        let p1 = PartialStep::text("xxx", "foo", CaptureType::Text);
        let p2 = PartialStep::text("xxx", "foo", CaptureType::Text);
        assert_eq!(p1, p2);
    }

    #[test]
    fn different_captured_texts_dont_match() {
        let p1 = PartialStep::text("xxx", "foo", CaptureType::Text);
        let p2 = PartialStep::text("xxx", "bar", CaptureType::Text);
        assert_ne!(p1, p2);
    }

    #[test]
    fn differently_named_captured_texts_dont_match() {
        let p1 = PartialStep::text("xxx", "foo", CaptureType::Text);
        let p2 = PartialStep::text("yyy", "foo", CaptureType::Text);
        assert_ne!(p1, p2);
    }

    #[test]
    fn differently_captured_texts_dont_match() {
        let p1 = PartialStep::uncaptured("foo");
        let p2 = PartialStep::text("xxx", "foo", CaptureType::Text);
        assert_ne!(p1, p2);
    }
}

/// The text of a part of a scenario step.
///
/// This is used by both unmatched and matched parts of a step. This
/// will later grow to include more information for better error
/// messges etc.
#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct StepSnippet {
    text: String,
}

impl StepSnippet {
    /// Create a new snippet.
    pub fn new(text: &str) -> StepSnippet {
        StepSnippet {
            text: text.to_owned(),
        }
    }

    /// Return the text of the snippet.
    pub fn text(&self) -> &str {
        &self.text
    }
}

#[cfg(test)]
mod test {
    use crate::bindings::CaptureType;

    use super::PartialStep;

    #[test]
    fn returns_uncaptured() {
        let p = PartialStep::uncaptured("foo");
        match p {
            PartialStep::UncapturedText(s) => assert_eq!(s.text(), "foo"),
            _ => panic!("expected UncapturedText: {:?}", p),
        }
    }

    #[test]
    fn returns_text() {
        let p = PartialStep::text("xxx", "foo", crate::bindings::CaptureType::Text);
        match p {
            PartialStep::CapturedText { name, text, kind } => {
                assert_eq!(name, "xxx");
                assert_eq!(text, "foo");
                assert_eq!(kind, CaptureType::Text);
            }
            _ => panic!("expected CapturedText: {:?}", p),
        }
    }
}
