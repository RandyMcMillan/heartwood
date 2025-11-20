use crate::{html::Location, SubplotError};
use serde::{Deserialize, Serialize};
use std::fmt;

/// A scenario step.
///
/// The scenario parser creates these kinds of data structures to
/// represent the parsed scenario step. The step consists of a kind
/// (expressed as a StepKind), and the text of the step.
///
/// This is just the step as it appears in the scenario in the input
/// text. It has not been matched with a binding. See MatchedStep for
/// that.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ScenarioStep {
    kind: StepKind,
    keyword: String,
    text: String,
    origin: Location,
}

impl ScenarioStep {
    /// Construct a new step.
    pub fn new(kind: StepKind, keyword: &str, text: &str, origin: Location) -> ScenarioStep {
        ScenarioStep {
            kind,
            keyword: keyword.to_owned(),
            text: text.to_owned(),
            origin,
        }
    }

    /// Return the kind of a step.
    pub fn kind(&self) -> StepKind {
        self.kind
    }

    /// Return the actual textual keyword of a step.
    pub fn keyword(&self) -> &str {
        &self.keyword
    }

    /// Return the text of a step.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Construct a step from a line in a scenario.
    ///
    /// If the step uses the "and" or "but" keyword, use the default
    /// step kind instead.
    pub fn new_from_str(
        text: &str,
        default: Option<StepKind>,
        origin: Location,
    ) -> Result<ScenarioStep, SubplotError> {
        if text.trim_start() != text {
            return Err(SubplotError::NotAtBoln(text.into()));
        }

        let mut words = text.split_ascii_whitespace();

        let keyword = match words.next() {
            Some(s) => s,
            _ => return Err(SubplotError::NoStepKeyword(text.to_string())),
        };

        let kind = match keyword.to_ascii_lowercase().as_str() {
            "given" => StepKind::Given,
            "when" => StepKind::When,
            "then" => StepKind::Then,
            "and" => default.ok_or(SubplotError::ContinuationTooEarly)?,
            "but" => default.ok_or(SubplotError::ContinuationTooEarly)?,
            _ => return Err(SubplotError::UnknownStepKind(keyword.to_string())),
        };

        let mut joined = String::new();
        for word in words {
            joined.push_str(word);
            joined.push(' ');
        }
        if joined.len() > 1 {
            joined.pop();
        }
        Ok(ScenarioStep::new(kind, keyword, &joined, origin))
    }

    pub(crate) fn origin(&self) -> &Location {
        &self.origin
    }
}

impl fmt::Display for ScenarioStep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.keyword(), self.text())
    }
}

/// Parse a scenario snippet into a vector of steps.
pub(crate) fn parse_scenario_snippet(
    text: &str,
    loc: &Location,
) -> Result<Vec<ScenarioStep>, SubplotError> {
    let mut steps = vec![];
    let mut prevkind = None;
    for (idx, line) in text.lines().enumerate() {
        let line_loc = match loc.clone() {
            Location::Known {
                filename,
                line,
                col,
            } => Location::Known {
                filename,
                line: line + idx,
                col,
            },
            Location::Unknown => Location::Unknown,
        };
        if !line.trim().is_empty() {
            let step = ScenarioStep::new_from_str(line, prevkind, line_loc)?;
            prevkind = Some(step.kind());
            steps.push(step);
        }
    }
    Ok(steps)
}

#[cfg(test)]
mod test_steps_parser {
    use super::{parse_scenario_snippet, Location, ScenarioStep, StepKind, SubplotError};
    use std::path::Path;

    fn parse(text: &str) -> Result<Vec<ScenarioStep>, SubplotError> {
        let loc = Location::new(Path::new("test"), 1, 1);
        parse_scenario_snippet(text, &loc)
    }

    #[test]
    fn empty_string() {
        assert_eq!(parse("").unwrap(), vec![]);
    }

    #[test]
    fn simple() {
        assert_eq!(
            parse("given foo").unwrap(),
            vec![ScenarioStep::new(
                StepKind::Given,
                "given",
                "foo",
                Location::new(Path::new("test"), 1, 1),
            )]
        );
    }

    #[test]
    fn two_simple() {
        assert_eq!(
            parse("given foo\nthen bar\n").unwrap(),
            vec![
                ScenarioStep::new(
                    StepKind::Given,
                    "given",
                    "foo",
                    Location::new(Path::new("test"), 1, 1),
                ),
                ScenarioStep::new(
                    StepKind::Then,
                    "then",
                    "bar",
                    Location::new(Path::new("test"), 2, 1),
                )
            ]
        );
    }

    #[test]
    fn preserve_nonascii_whitespace() {
        assert_eq!(
            parse("given a\u{00a0}hard-space").unwrap(),
            vec![ScenarioStep::new(
                StepKind::Given,
                "given",
                "a\u{00a0}hard-space",
                Location::new(Path::new("test"), 1, 1),
            )]
        );
        match parse("given\u{00a0}wrong hard-space position")
            .err()
            .unwrap()
        {
            SubplotError::UnknownStepKind(s) if s == "given\u{00a0}wrong" => {
                // Success
            }
            e => {
                panic!("{e:?}");
            }
        }
    }
}

/// The kind of scenario step we have: given, when, or then.
///
/// This needs to be extended if the Subplot language gets extended with other
/// kinds of steps. However, note that the scenario parser will hide aliases,
/// such as "and" to mean the same kind as the previous step.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum StepKind {
    /// A "given some precondition" step.
    Given,

    /// A "when something happens" step.
    When,

    /// A "then some condition" step.
    Then,
}

impl fmt::Display for StepKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            StepKind::Given => "given",
            StepKind::When => "when",
            StepKind::Then => "then",
        };
        write!(f, "{s}")
    }
}

#[cfg(test)]
mod test {
    use crate::html::Location;

    use super::{ScenarioStep, StepKind, SubplotError};

    #[test]
    fn parses_given() {
        let step =
            ScenarioStep::new_from_str("GIVEN I am Tomjon", None, Location::Unknown).unwrap();
        assert_eq!(step.kind(), StepKind::Given);
        assert_eq!(step.text(), "I am Tomjon");
    }

    #[test]
    fn parses_given_with_extra_spaces() {
        let step =
            ScenarioStep::new_from_str("given     I   am   Tomjon   ", None, Location::Unknown)
                .unwrap();
        assert_eq!(step.kind(), StepKind::Given);
        assert_eq!(step.text(), "I am Tomjon");
    }

    #[test]
    fn parses_when() {
        let step =
            ScenarioStep::new_from_str("when I declare myself king", None, Location::Unknown)
                .unwrap();
        assert_eq!(step.kind(), StepKind::When);
        assert_eq!(step.text(), "I declare myself king");
    }

    #[test]
    fn parses_then() {
        let step = ScenarioStep::new_from_str("thEN everyone accepts it", None, Location::Unknown)
            .unwrap();
        assert_eq!(step.kind(), StepKind::Then);
        assert_eq!(step.text(), "everyone accepts it");
    }

    #[test]
    fn parses_and() {
        let step = ScenarioStep::new_from_str(
            "and everyone accepts it",
            Some(StepKind::Then),
            Location::Unknown,
        )
        .unwrap();
        assert_eq!(step.kind(), StepKind::Then);
        assert_eq!(step.text(), "everyone accepts it");
    }

    #[test]
    fn fails_to_parse_and() {
        let step = ScenarioStep::new_from_str("and everyone accepts it", None, Location::Unknown);
        assert!(step.is_err());
        match step.err() {
            None => unreachable!(),
            Some(SubplotError::ContinuationTooEarly) => (),
            Some(e) => panic!("Incorrect error: {}", e),
        }
    }
}
