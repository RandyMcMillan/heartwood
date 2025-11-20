use super::MatchedStep;
use super::MatchedSteps;
use super::PartialStep;
use super::ScenarioStep;
use super::StepKind;
use crate::Warning;
use crate::{resource, SubplotError};

use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::fmt::{Debug, Write};
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;

use lazy_static::lazy_static;
use log::trace;
use regex::{escape, Regex, RegexBuilder};

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
/// The type of a scenario step capture
///
/// Each scenario step is given a particular type, these types are defined by
/// the binding for the step, either by means of simple patterns, or by
/// the types map in the binding.
pub enum CaptureType {
    /// A word is a sequence of non-whitespace characters and is the default
    /// capture type.
    Word,

    /// Text is simply any sequence of characters.  Typically used at the end
    /// of a capture string.
    Text,

    /// Integers are optionally negative numbers with no decimal part.
    Int,

    /// Uints are integers with no sign marker permitted.
    Uint,

    /// Numbers are optionally negative sequences of digits with an optional
    /// decimal point and subsequent decimal digits.
    Number,

    /// Files are words which are special in that they have to match a filename
    /// for one of the embedded files in the document, otherwise codegen will
    /// refuse to run.
    File,

    /// Paths are sequences of non-whitespace characters which will be interpreted
    /// as a file-path (e.g. PathBuf) or similar when processed by a step
    Path,
}

impl FromStr for CaptureType {
    type Err = SubplotError;

    fn from_str(value: &str) -> Result<Self, SubplotError> {
        match value.to_ascii_lowercase().as_str() {
            "word" => Ok(Self::Word),
            "text" => Ok(Self::Text),
            "int" => Ok(Self::Int),
            "uint" => Ok(Self::Uint),
            "number" => Ok(Self::Number),
            "file" => Ok(Self::File),
            "path" => Ok(Self::Path),
            _ => Err(SubplotError::UnknownTypeInBinding(value.to_string())),
        }
    }
}

impl CaptureType {
    /// Retrieve the string representation of this capture type
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Word => "word",
            Self::Text => "text",
            Self::Int => "int",
            Self::Uint => "uint",
            Self::Number => "number",
            Self::File => "file",
            Self::Path => "path",
        }
    }

    /// Retrieve the regular expression representation of this capture type
    pub fn regex_str(self) -> &'static str {
        match self {
            Self::Word => r"\S+",
            Self::Text => r".*",
            Self::Int => r"-?\d+",
            Self::Uint => r"\d+",
            Self::Number => r"-?\d+(\.\d+)?",
            Self::File => r"\S+",
            Self::Path => r"\S+",
        }
    }
}

/// A link from a binding to its implementation in a given language.
///
/// Such a link comprises a function name to call for the step and
/// an optional function name to call to clean the step up at the end.
#[derive(Debug, Clone)]
pub struct BindingImpl {
    function: String,
    cleanup: Option<String>,
}

impl BindingImpl {
    /// Create a new binding implementation
    ///
    /// ```ignore
    /// # use subplot::bindings::BindingImpl;
    ///
    /// let bimpl = BindingImpl::new("foo::bar::func", Some("foo::bar::func_cleanup"));
    /// ```
    pub fn new(function: &str, cleanup: Option<&str>) -> Self {
        Self {
            function: function.to_string(),
            cleanup: cleanup.map(str::to_string),
        }
    }

    /// Retrieve the function name in this binding
    ///
    /// ```ignore
    /// # use subplot::bindings::BindingImpl;
    /// let bimpl = BindingImpl::new("foo::bar::func", None);
    ///
    /// assert_eq!(bimpl.function(), "foo::bar::func");
    /// ```
    pub fn function(&self) -> &str {
        &self.function
    }

    /// Retrieve the cleanup function name in this binding
    ///
    /// ```ignore
    /// # use subplot::bindings::BindingImpl;
    /// let bimpl = BindingImpl::new("foo::bar::func", None);
    ///
    /// assert_eq!(bimpl.cleanup(), None);
    ///
    /// let bimpl = BindingImpl::new("foo::bar::func", Some("foo::bar::func_cleanup"));
    ///
    /// assert_eq!(bimpl.cleanup(), Some("foo::bar::func_cleanup"));
    /// ```
    pub fn cleanup(&self) -> Option<&str> {
        self.cleanup.as_deref()
    }
}

/// A binding of a scenario step to its implementation.
///
/// Contains the pattern used to match against scenario steps,
/// combined with the step kind. The pattern is a regular expression
/// as understood by the regex crate.
#[derive(Debug, Clone)]
pub struct Binding {
    kind: StepKind,
    pattern: String,
    regex: Regex,
    impls: HashMap<String, Arc<BindingImpl>>,
    types: HashMap<String, CaptureType>,
    doc: Option<String>,
    filename: Arc<Path>,
}

impl Binding {
    /// Create a new Binding, from a step kind and a pattern.
    pub fn new(
        kind: StepKind,
        pattern: &str,
        case_sensitive: bool,
        types: HashMap<String, CaptureType>,
        doc: Option<String>,
        filename: Arc<Path>,
    ) -> Result<Binding, SubplotError> {
        let regex = RegexBuilder::new(&format!("^{pattern}$"))
            .case_insensitive(!case_sensitive)
            .build()
            .map_err(|err| SubplotError::Regex(pattern.to_string(), err))?;

        Ok(Binding {
            kind,
            pattern: pattern.to_owned(),
            regex,
            impls: HashMap::new(),
            types,
            doc,
            filename,
        })
    }

    /// Insert an impl into this binding
    pub fn add_impl(&mut self, template: &str, function: &str, cleanup: Option<&str>) {
        self.impls.insert(
            template.to_string(),
            Arc::new(BindingImpl::new(function, cleanup)),
        );
    }

    /// Return the kind of step the binding is for.
    pub fn kind(&self) -> StepKind {
        self.kind
    }

    /// Return text of pattern.
    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    /// Return documentation string for binding, if any.
    pub fn doc(&self) -> Option<&str> {
        self.doc.as_deref()
    }

    /// Retrieve a particular implementation by name
    pub fn step_impl(&self, template: &str) -> Option<Arc<BindingImpl>> {
        self.impls.get(template).cloned()
    }

    /// Return the compiled regular expression for the pattern of the
    /// binding.
    ///
    /// The regular expression matches the whole text of a scenario step.
    pub fn regex(&self) -> &Regex {
        &self.regex
    }

    /// Return the type bindings for this binding.
    pub fn types(&self) -> impl Iterator<Item = (&str, CaptureType)> {
        self.types.iter().map(|(s, c)| (s.as_str(), *c))
    }

    /// Try to match defined binding against a parsed scenario step.
    pub fn match_with_step(&self, template: &str, step: &ScenarioStep) -> Option<MatchedStep> {
        if self.kind() != step.kind() {
            return None;
        }

        let step_text = step.text();
        let caps = self.regex.captures(step_text)?;

        // If there is only one capture, it's the whole string.
        let mut m = MatchedStep::new(self, template, step.origin().clone());
        if caps.len() == 1 {
            m.append_part(PartialStep::uncaptured(step_text));
            return Some(m);
        }

        // Otherwise, return captures as PartialStep::Text, and the
        // surrounding text as PartialStep::UnmatchedText.
        let mut prev_end = 0;
        for cap in caps.iter().skip(1).flatten() {
            if cap.start() > prev_end {
                let part = PartialStep::uncaptured(&step_text[prev_end..cap.start()]);
                m.append_part(part);
            }

            // Find name for capture.
            let mut capname: Option<&str> = None;
            for name in self.regex.capture_names().flatten() {
                if let Some(mm) = caps.name(name) {
                    if mm.start() == cap.start() && mm.end() == cap.end() {
                        capname = Some(name);
                    }
                }
            }

            let part = match capname {
                None => PartialStep::uncaptured(&step_text[prev_end..cap.start()]),
                Some(name) => {
                    // Before continuing, verify that the capture matches the
                    // pattern for this capture
                    let cap = cap.as_str();
                    // These unwraps are safe because we ensured the map is complete
                    // in the constructor, and that all the types are known.
                    let kind = self.types.get(name).copied().unwrap_or(CaptureType::Text);
                    let rx = KIND_PATTERNS.get(&kind).unwrap();
                    if !rx.is_match(cap) {
                        // This capture doesn't match the kind so it's not
                        // valid for this binding.
                        return None;
                    }
                    PartialStep::text(name, cap, kind)
                }
            };

            m.append_part(part);
            prev_end = cap.end();
        }

        // There might be unmatched text at the end.
        if prev_end < step_text.len() {
            let part = PartialStep::uncaptured(&step_text[prev_end..]);
            m.append_part(part);
        }

        Some(m)
    }

    fn filename(&self) -> &Path {
        &self.filename
    }

    fn check(&self, warnings: &mut crate::Warnings) -> Result<(), SubplotError> {
        fn nth(i: usize) -> &'static str {
            match i % 10 {
                1 => "st",
                2 => "nd",
                3 => "rd",
                _ => "th",
            }
        }
        for (nr, capture) in self.regex.capture_names().enumerate().skip(1) {
            if let Some(name) = capture {
                if !self.types.contains_key(name) {
                    warnings.push(Warning::MissingCaptureType(
                        self.filename().to_owned(),
                        format!("{}: {}", self.kind(), self.pattern()),
                        name.to_string(),
                    ));
                }
            } else {
                warnings.push(Warning::MissingCaptureName(
                    self.filename().to_owned(),
                    format!("{}: {}", self.kind(), self.pattern()),
                    format!("{nr}{}", nth(nr)),
                ));
            }
        }
        Ok(())
    }
}

impl PartialEq for Binding {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind && self.pattern == other.pattern
    }
}

impl Eq for Binding {}

#[cfg(test)]
mod test_binding {
    use super::{Binding, CaptureType};
    use crate::html::Location;
    use crate::PartialStep;
    use crate::ScenarioStep;
    use crate::StepKind;
    use std::collections::HashMap;
    use std::path::Path;
    use std::path::PathBuf;
    use std::sync::Arc;

    fn path() -> Arc<Path> {
        PathBuf::new().into()
    }

    #[test]
    fn creates_new() {
        let b = Binding::new(
            StepKind::Given,
            "I am Tomjon",
            false,
            HashMap::new(),
            None,
            path(),
        )
        .unwrap();
        assert_eq!(b.kind(), StepKind::Given);
        assert!(b.regex().is_match("I am Tomjon"));
        assert!(!b.regex().is_match("I am Tomjon of Lancre"));
        assert!(!b.regex().is_match("Hello, I am Tomjon"));
    }

    #[test]
    fn equal() {
        let a = Binding::new(
            StepKind::Given,
            "I am Tomjon",
            false,
            HashMap::new(),
            None,
            path(),
        )
        .unwrap();
        let b = Binding::new(
            StepKind::Given,
            "I am Tomjon",
            false,
            HashMap::new(),
            None,
            path(),
        )
        .unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn not_equal() {
        let a = Binding::new(
            StepKind::Given,
            "I am Tomjon",
            false,
            HashMap::new(),
            None,
            path(),
        )
        .unwrap();
        let b = Binding::new(
            StepKind::Given,
            "I am Tomjon of Lancre",
            false,
            HashMap::new(),
            None,
            path(),
        )
        .unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn does_not_match_with_wrong_kind() {
        let step = ScenarioStep::new(StepKind::Given, "given", "yo", Location::Unknown);
        let b = Binding::new(StepKind::When, "yo", false, HashMap::new(), None, path()).unwrap();
        assert!(b.match_with_step("", &step).is_none());
    }

    #[test]
    fn does_not_match_with_wrong_text() {
        let step = ScenarioStep::new(StepKind::Given, "given", "foo", Location::Unknown);
        let b = Binding::new(StepKind::Given, "bar", false, HashMap::new(), None, path()).unwrap();
        assert!(b.match_with_step("", &step).is_none());
    }

    #[test]
    fn match_with_fixed_pattern() {
        let step = ScenarioStep::new(StepKind::Given, "given", "foo", Location::Unknown);
        let b = Binding::new(StepKind::Given, "foo", false, HashMap::new(), None, path()).unwrap();
        let m = b.match_with_step("", &step).unwrap();
        assert_eq!(m.kind(), StepKind::Given);
        let mut parts = m.parts();
        let p = parts.next().unwrap();
        assert_eq!(p, &PartialStep::uncaptured("foo"));
        assert_eq!(parts.next(), None);
    }

    #[test]
    fn match_with_regex() {
        let step = ScenarioStep::new(
            StepKind::Given,
            "given",
            "I am Tomjon, I am",
            Location::Unknown,
        );
        let b = Binding::new(
            StepKind::Given,
            r"I am (?P<who>\S+), I am",
            false,
            HashMap::new(),
            None,
            path(),
        )
        .unwrap();
        let m = b.match_with_step("", &step).unwrap();
        assert_eq!(m.kind(), StepKind::Given);
        let mut parts = m.parts();
        assert_eq!(parts.next().unwrap(), &PartialStep::uncaptured("I am "));
        assert_eq!(
            parts.next().unwrap(),
            &PartialStep::text("who", "Tomjon", CaptureType::Text)
        );
        assert_eq!(parts.next().unwrap(), &PartialStep::uncaptured(", I am"));
        assert_eq!(parts.next(), None);
    }

    #[test]
    fn case_sensitive_mismatch() {
        let step = ScenarioStep::new(StepKind::Given, "given", "I am Tomjon", Location::Unknown);
        let b = Binding::new(
            StepKind::Given,
            r"i am tomjon",
            false,
            HashMap::new(),
            None,
            path(),
        )
        .unwrap();
        assert!(b.match_with_step("", &step).is_some());
        let b = Binding::new(
            StepKind::Given,
            r"i am tomjon",
            true,
            HashMap::new(),
            None,
            path(),
        )
        .unwrap();
        assert!(b.match_with_step("", &step).is_none());
    }
}

/// Set of all known bindings.
#[derive(Debug, Default)]
pub struct Bindings {
    bindings: Vec<Binding>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ParsedImpl {
    function: String,
    cleanup: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ParsedBinding {
    given: Option<String>,
    when: Option<String>,
    then: Option<String>,
    #[serde(default, rename = "impl")]
    impls: HashMap<String, ParsedImpl>,
    regex: Option<bool>,
    #[serde(default)]
    case_sensitive: bool,
    #[serde(default)]
    types: HashMap<String, CaptureType>,
    doc: Option<String>,
}

impl Bindings {
    /// Create a new, empty set of bindings.
    pub fn new() -> Bindings {
        Bindings::default()
    }

    /// Return number of bindings in set.
    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    /// Are there no bindings?
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }

    /// Add a binding to the set.
    pub fn add(&mut self, binding: Binding) {
        self.bindings.push(binding);
    }

    /// Add bindings from a YAML string
    pub fn add_from_yaml(&mut self, yaml: &str, filename: Arc<Path>) -> Result<(), SubplotError> {
        let bindings: Vec<ParsedBinding> = marked_yaml::from_yaml_with_options(
            0, /* TODO: Keep track of sources */
            yaml,
            marked_yaml::LoaderOptions::default()
                .toplevel_sequence()
                .lowercase_keys(true),
        )
        .map_err(SubplotError::Metadata)?;
        for binding in bindings {
            self.add(from_hashmap(&binding, Arc::clone(&filename))?);
        }
        Ok(())
    }

    /// Return slice of all bindings.
    pub fn bindings(&self) -> &[Binding] {
        &self.bindings
    }

    /// Find the binding matching a given scenario step, if there is
    /// exactly one.
    pub fn find(&self, template: &str, step: &ScenarioStep) -> Result<MatchedStep, SubplotError> {
        let mut matches: Vec<MatchedStep> = self
            .bindings()
            .iter()
            .filter_map(|b| b.match_with_step(template, step))
            .collect();
        if matches.len() > 1 {
            // Too many matching bindings.
            Err(SubplotError::BindingNotUnique(
                step.to_string(),
                MatchedSteps::new(matches),
            ))
        } else if let Some(m) = matches.pop() {
            // Exactly one matching binding.
            Ok(m)
        } else {
            // No matching bindings.
            Err(SubplotError::BindingUnknown(step.to_string()))
        }
    }

    /// Add bindings from a file.
    pub fn add_from_file<P>(
        &mut self,
        filename: P,
        template: Option<&str>,
    ) -> Result<(), SubplotError>
    where
        P: AsRef<Path> + Debug,
    {
        let filename = filename.as_ref();
        let yaml = resource::read_as_string(filename, template)
            .map_err(|e| SubplotError::BindingsFileNotFound(filename.into(), e))?;
        trace!("Loaded file content");
        self.add_from_yaml(&yaml, filename.to_owned().into())
            .map_err(|e| SubplotError::BindingFileParseError(filename.to_owned(), Box::new(e)))?;
        Ok(())
    }

    /// Is there a binding for a given raw step?
    pub fn has(&self, kind: StepKind, pattern: &str) -> bool {
        let m = self
            .bindings
            .iter()
            .filter(|b| b.kind() == kind && b.pattern() == pattern);
        m.count() == 1
    }

    /// Check these bindings for any warnings which users might need to know about
    pub fn check(&self, warnings: &mut crate::Warnings) -> Result<(), SubplotError> {
        for binding in self.bindings() {
            binding.check(warnings)?;
        }
        Ok(())
    }
}

fn from_hashmap(parsed: &ParsedBinding, filename: Arc<Path>) -> Result<Binding, SubplotError> {
    let given: i32 = parsed.given.is_some().into();
    let when: i32 = parsed.when.is_some().into();
    let then: i32 = parsed.then.is_some().into();

    if given + when + then == 0 {
        let msg = format!("{parsed:?}");
        return Err(SubplotError::BindingWithoutKnownKeyword(msg));
    }

    if given + when + then > 1 {
        let msg = format!("{parsed:?}");
        return Err(SubplotError::BindingHasManyKeywords(msg));
    }

    let (kind, pattern) = if parsed.given.is_some() {
        (StepKind::Given, parsed.given.as_ref().unwrap())
    } else if parsed.when.is_some() {
        (StepKind::When, parsed.when.as_ref().unwrap())
    } else if parsed.then.is_some() {
        (StepKind::Then, parsed.then.as_ref().unwrap())
    } else {
        let msg = format!("{parsed:?}");
        return Err(SubplotError::BindingWithoutKnownKeyword(msg));
    };

    let mut types = parsed.types.clone();

    let pattern = if parsed.regex.unwrap_or(false) {
        pattern.to_string()
    } else {
        // if we get here parsed.regex is either None or Some(false)
        regex_from_simple_pattern(pattern, parsed.regex.is_some(), &mut types)?
    };

    trace!("Successfully acquired binding");

    let mut ret = Binding::new(
        kind,
        &pattern,
        parsed.case_sensitive,
        types,
        parsed.doc.clone(),
        filename,
    )?;
    trace!("Binding parsed OK");
    for (template, pimpl) in &parsed.impls {
        ret.add_impl(template, &pimpl.function, pimpl.cleanup.as_deref());
    }

    Ok(ret)
}

#[cfg(test)]
mod test_bindings {
    use crate::bindings::CaptureType;
    use crate::html::Location;
    use crate::Binding;
    use crate::Bindings;
    use crate::PartialStep;
    use crate::ScenarioStep;
    use crate::StepKind;
    use crate::SubplotError;

    use std::collections::HashMap;
    use std::path::Path;
    use std::path::PathBuf;
    use std::sync::Arc;

    fn path() -> Arc<Path> {
        PathBuf::new().into()
    }

    #[test]
    fn has_no_bindings_initially() {
        let bindings = Bindings::new();
        assert_eq!(bindings.bindings().len(), 0);
    }

    #[test]
    fn adds_binding() {
        let binding = Binding::new(
            StepKind::Given,
            r"I am (?P<name>\S+)",
            false,
            HashMap::new(),
            None,
            path(),
        )
        .unwrap();
        let mut bindings = Bindings::new();
        bindings.add(binding.clone());
        assert_eq!(bindings.bindings(), &[binding]);
    }

    #[test]
    fn adds_from_yaml() {
        let yaml = r#"
- GIVEN: I am Tomjon
  impl:
    python:
      function: set_name
- when: I declare myself king
  impl:
    python:
      Function: declare_king
- tHEn: there is applause
  impl:
    python:
      function: check_for_applause
- given: you are alice
  impl:
    python:
      function: other_name
  case_sensitive: true
- then: the total is {total}
  impl:
    python:
      function: check_total
  types:
    total: word
"#;
        let mut bindings = Bindings::new();
        bindings.add_from_yaml(yaml, path()).unwrap();
        println!("test: {bindings:?}");
        assert!(bindings.has(StepKind::Given, "I am Tomjon"));
        assert!(bindings.has(StepKind::When, "I declare myself king"));
        assert!(bindings.has(StepKind::Then, "there is applause"));
        assert!(bindings.has(StepKind::Given, "you are alice"));
        assert!(!bindings.has(StepKind::Given, "you are Alice"));
        assert!(bindings.has(StepKind::Then, "the total is (?P<total>\\S+)"));
        assert_eq!(bindings.len(), 5);
    }

    #[test]
    fn add_from_yaml_notices_multiple_keywords() {
        let yaml = "
- Given: I am Tomjon
  wheN: I am indeed Tomjon
  impl:
    python:
      FUNCTION: set_name
";
        match Bindings::new().add_from_yaml(yaml, path()) {
            Ok(_) => unreachable!(),
            Err(SubplotError::BindingHasManyKeywords(_)) => (),
            Err(e) => panic!("Incorrect error: {}", e),
        }
    }

    #[test]
    fn typemap_must_match_pattern() {
        let yaml = "
- then: you are {age:word} years old
  impl:
    python:
      function: check_age
  types:
    age: number
";
        match Bindings::new().add_from_yaml(yaml, path()) {
            Ok(_) => unreachable!(),
            Err(SubplotError::SimplePatternKindMismatch(_)) => (),
            Err(e) => panic!("Incorrect error: {}", e),
        }
    }

    #[test]
    fn does_not_find_match_for_unmatching_kind() {
        let step = ScenarioStep::new(StepKind::Given, "given", "I am Tomjon", Location::Unknown);
        let binding = Binding::new(
            StepKind::When,
            r"I am Tomjon",
            false,
            HashMap::new(),
            None,
            path(),
        )
        .unwrap();
        let mut bindings = Bindings::new();
        bindings.add(binding);
        assert!(matches!(
            bindings.find("", &step),
            Err(SubplotError::BindingUnknown(_))
        ));
    }

    #[test]
    fn does_not_find_match_for_unmatching_pattern() {
        let step = ScenarioStep::new(StepKind::Given, "given", "I am Tomjon", Location::Unknown);
        let binding = Binding::new(
            StepKind::Given,
            r"I am Tomjon of Lancre",
            false,
            HashMap::new(),
            None,
            path(),
        )
        .unwrap();
        let mut bindings = Bindings::new();
        bindings.add(binding);
        assert!(matches!(
            bindings.find("", &step),
            Err(SubplotError::BindingUnknown(_))
        ));
    }

    #[test]
    fn two_matching_bindings() {
        let step = ScenarioStep::new(StepKind::Given, "given", "I am Tomjon", Location::Unknown);
        let mut bindings = Bindings::default();
        bindings.add(
            Binding::new(
                StepKind::Given,
                r"I am Tomjon",
                false,
                HashMap::new(),
                None,
                path(),
            )
            .unwrap(),
        );
        bindings.add(
            Binding::new(
                StepKind::Given,
                &super::regex_from_simple_pattern(r"I am {name}", false, &mut HashMap::new())
                    .unwrap(),
                false,
                HashMap::new(),
                None,
                path(),
            )
            .unwrap(),
        );
        assert!(matches!(
            bindings.find("", &step),
            Err(SubplotError::BindingNotUnique(_, _))
        ));
    }

    #[test]
    fn finds_match_for_fixed_string_pattern() {
        let step = ScenarioStep::new(StepKind::Given, "given", "I am Tomjon", Location::Unknown);
        let binding = Binding::new(
            StepKind::Given,
            r"I am Tomjon",
            false,
            HashMap::new(),
            None,
            path(),
        )
        .unwrap();
        let mut bindings = Bindings::new();
        bindings.add(binding);
        let m = bindings.find("", &step).unwrap();
        assert_eq!(m.kind(), StepKind::Given);
        let mut parts = m.parts();
        let p = parts.next().unwrap();
        match p {
            PartialStep::UncapturedText(t) => assert_eq!(t.text(), "I am Tomjon"),
            _ => panic!("unexpected part: {:?}", p),
        }
        assert_eq!(parts.next(), None);
    }

    #[test]
    fn finds_match_for_regexp_pattern() {
        let step = ScenarioStep::new(StepKind::Given, "given", "I am Tomjon", Location::Unknown);
        let binding = Binding::new(
            StepKind::Given,
            r"I am (?P<name>\S+)",
            false,
            HashMap::new(),
            None,
            path(),
        )
        .unwrap();
        let mut bindings = Bindings::new();
        bindings.add(binding);
        let m = bindings.find("", &step).unwrap();
        assert_eq!(m.kind(), StepKind::Given);
        let mut parts = m.parts();
        let p = parts.next().unwrap();
        match p {
            PartialStep::UncapturedText(t) => assert_eq!(t.text(), "I am "),
            _ => panic!("unexpected part: {:?}", p),
        }
        let p = parts.next().unwrap();
        match p {
            PartialStep::CapturedText { name, text, kind } => {
                assert_eq!(name, "name");
                assert_eq!(text, "Tomjon");
                assert_eq!(kind, &CaptureType::Text);
            }
            _ => panic!("unexpected part: {:?}", p),
        }
        assert_eq!(parts.next(), None);
    }
}

lazy_static! {
    static ref KIND_PATTERNS: HashMap<CaptureType, Regex> = {
        let mut map = HashMap::new();
        for ty in ([
            CaptureType::Word,
            CaptureType::Text,
            CaptureType::Int,
            CaptureType::Uint,
            CaptureType::Number,
            CaptureType::File,
            CaptureType::Path,
        ]).iter().copied() {
            // This Unwrap is okay because we shouldn't have any bugs in the
            // regular expressions here, and if we did, it'd be bad for everyone
            // and caught in the test suite anyway.
            let rx = Regex::new(&format!("^{}$", ty.regex_str())).unwrap();
            map.insert(ty, rx);
        }
        map
    };
}

fn regex_from_simple_pattern(
    pattern: &str,
    explicit_plain: bool,
    types: &mut HashMap<String, CaptureType>,
) -> Result<String, SubplotError> {
    let pat = Regex::new(r"\{[^\s\{\}]+\}").unwrap();
    let mut r = String::new();
    let mut end = 0;
    for m in pat.find_iter(pattern) {
        let before = &pattern[end..m.start()];
        if before.find('{').is_some() || before.find('}').is_some() {
            return Err(SubplotError::StrayBraceInSimplePattern(pattern.to_string()));
        }
        if !explicit_plain && before.chars().any(|c| r"$^*.()+\?|[]".contains(c)) {
            return Err(SubplotError::SimplePatternHasMetaCharacters(
                pattern.to_owned(),
            ));
        }
        r.push_str(&escape(before));
        let name = &pattern[m.start() + 1..m.end() - 1];

        let (name, kind) = if let Some(i) = name.find(':') {
            let (name, suffix) = name.split_at(i);
            assert!(suffix.starts_with(':'));
            let kind = &suffix[1..];
            let kind = CaptureType::from_str(kind)?;
            (name, Some(kind))
        } else {
            (name, None)
        };

        let (name, kind) = match (name, kind, types.contains_key(name)) {
            (name, Some(kind), false) => {
                // There is a kind, but it's not in the map
                types.insert(name.to_string(), kind);
                (name, kind)
            }
            (name, None, true) => {
                // There is no kind, but it is present in the map
                (name, types[name])
            }
            (name, Some(kind), true) => {
                // There is a kind and it's in the map, they must match
                if kind != *types.get(name).unwrap() {
                    return Err(SubplotError::SimplePatternKindMismatch(name.to_string()));
                }
                (name, kind)
            }
            (name, None, false) => {
                // There is no kind, and it's not in the map, so default to word
                types.insert(name.to_string(), CaptureType::Word);
                (name, CaptureType::Word)
            }
        };

        write!(r, r"(?P<{}>{})", name, kind.regex_str()).map_err(SubplotError::StringFormat)?;
        end = m.end();
    }
    let after = &pattern[end..];
    if after.find('{').is_some() || after.find('}').is_some() {
        return Err(SubplotError::StrayBraceInSimplePattern(pattern.to_string()));
    }
    if !explicit_plain && after.chars().any(|c| r"$^*.()+\?|[]".contains(c)) {
        return Err(SubplotError::SimplePatternHasMetaCharacters(
            pattern.to_owned(),
        ));
    }
    r.push_str(&escape(after));
    Ok(r)
}

#[cfg(test)]
mod test_regex_from_simple_pattern {
    use super::{regex_from_simple_pattern, CaptureType};
    use crate::SubplotError;
    use regex::Regex;
    use std::collections::HashMap;

    #[test]
    fn returns_empty_string_as_is() {
        let ret = regex_from_simple_pattern("", false, &mut HashMap::new()).unwrap();
        assert_eq!(ret, "");
    }

    #[test]
    fn returns_boring_pattern_as_is() {
        let ret = regex_from_simple_pattern("boring", false, &mut HashMap::new()).unwrap();
        assert_eq!(ret, "boring");
    }

    #[test]
    fn returns_pattern_with_regexp_chars_escaped() {
        let ret = regex_from_simple_pattern(r".[]*\\", true, &mut HashMap::new()).unwrap();
        assert_eq!(ret, r"\.\[\]\*\\\\");
    }

    fn matches(pattern: &str, text: &str) {
        let r = regex_from_simple_pattern(pattern, false, &mut HashMap::new()).unwrap();
        let r = Regex::new(&r).unwrap();
        let m = r.find(text);
        assert!(m.is_some());
        let m = m.unwrap();
        assert_eq!(m.start(), 0);
        assert_eq!(m.end(), text.len());
    }

    fn doesnt_match(pattern: &str, text: &str) {
        let r = regex_from_simple_pattern(pattern, false, &mut HashMap::new()).unwrap();
        let r = Regex::new(&r).unwrap();
        if let Some(m) = r.find(text) {
            assert!(m.start() > 0 || m.end() < text.len());
        }
    }

    #[test]
    fn kindless_simple_pattern() {
        let pattern = "{name}";
        matches(pattern, "Tomjon");
        doesnt_match(pattern, "Tomjon of Lancre");
    }

    #[test]
    fn simple_word_pattern() {
        let pattern = "{name:word}";
        matches(pattern, "Tomjon");
        doesnt_match(pattern, "Tomjon of Lancre");
    }

    #[test]
    fn simple_text_pattern() {
        let pattern = "{name:text}";
        matches(pattern, "Tomjon");
        matches(pattern, "");
        matches(pattern, "Tomjon of Lancre");
    }

    #[test]
    fn simple_int_pattern() {
        let pattern = "{foo:int}";
        matches(pattern, "0");
        matches(pattern, "-0");
        matches(pattern, "1");
        matches(pattern, "-1");
        matches(pattern, "1234");
        matches(pattern, "-1234");
        doesnt_match(pattern, " ");
        doesnt_match(pattern, "one ");
        doesnt_match(pattern, "1.2 ");
        doesnt_match(pattern, "-1.2 ");
    }

    #[test]
    fn simple_uint_pattern() {
        let pattern = "{foo:uint}";
        matches(pattern, "0");
        matches(pattern, "1");
        matches(pattern, "1234");
        doesnt_match(pattern, "-0");
        doesnt_match(pattern, "-1 ");
        doesnt_match(pattern, "-1234");
        doesnt_match(pattern, " ");
        doesnt_match(pattern, "one ");
        doesnt_match(pattern, "1.2 ");
        doesnt_match(pattern, "-1.2 ");
    }

    #[test]
    fn simple_number_pattern() {
        let pattern = "{foo:number}";
        matches(pattern, "0");
        matches(pattern, "-0");
        matches(pattern, "1");
        matches(pattern, "-1");
        matches(pattern, "1234");
        matches(pattern, "-1234");
        matches(pattern, "1.2");
        matches(pattern, "-1.2");
        doesnt_match(pattern, "");
        doesnt_match(pattern, " ");
        doesnt_match(pattern, "one");
    }

    #[test]
    fn returns_error_for_stray_opening_brace() {
        match regex_from_simple_pattern("{", false, &mut HashMap::new()) {
            Err(SubplotError::StrayBraceInSimplePattern(_)) => (),
            Err(e) => panic!("unexpected error: {}", e),
            _ => unreachable!(),
        }
    }

    #[test]
    fn returns_error_for_stray_closing_brace() {
        match regex_from_simple_pattern("}", false, &mut HashMap::new()) {
            Err(SubplotError::StrayBraceInSimplePattern(_)) => (),
            Err(e) => panic!("unexpected error: {}", e),
            _ => unreachable!(),
        }
    }

    #[test]
    fn returns_error_for_stray_opening_brace_before_capture() {
        match regex_from_simple_pattern("{{foo}", false, &mut HashMap::new()) {
            Err(SubplotError::StrayBraceInSimplePattern(_)) => (),
            Err(e) => panic!("unexpected error: {}", e),
            _ => unreachable!(),
        }
    }

    #[test]
    fn returns_error_for_stray_closing_brace_before_capture() {
        match regex_from_simple_pattern("}{foo}", false, &mut HashMap::new()) {
            Err(SubplotError::StrayBraceInSimplePattern(_)) => (),
            Err(e) => panic!("unexpected error: {}", e),
            _ => unreachable!(),
        }
    }

    #[test]
    fn typemap_updated_on_pattern_parse_default() {
        let mut types = HashMap::new();
        assert!(regex_from_simple_pattern("{foo}", false, &mut types).is_ok());
        assert!(matches!(types.get("foo"), Some(CaptureType::Word)));
    }

    #[test]
    fn typemap_checked_on_pattern_parse_and_default_agrees() {
        let mut types = HashMap::new();
        types.insert("foo".into(), "word".parse().unwrap());
        assert!(regex_from_simple_pattern("{foo}", false, &mut types).is_ok());
        assert_eq!(types.len(), 1);
        assert!(matches!(types.get("foo"), Some(CaptureType::Word)));
    }

    #[test]
    fn typemap_updated_on_pattern_parse_explicit() {
        let mut types = HashMap::new();
        assert!(regex_from_simple_pattern("{foo:number}", false, &mut types).is_ok());
        assert!(matches!(types.get("foo"), Some(CaptureType::Number)));
    }

    #[test]
    fn typemap_used_when_kind_not_present() {
        let mut types = HashMap::new();
        types.insert("foo".into(), "number".parse().unwrap());
        assert_eq!(
            regex_from_simple_pattern("{foo}", false, &mut types).unwrap(),
            r"(?P<foo>-?\d+(\.\d+)?)"
        );
    }

    #[test]
    fn typemap_and_pattern_kind_must_match() {
        let mut types = HashMap::new();
        types.insert("foo".into(), "number".parse().unwrap());
        assert!(matches!(
            regex_from_simple_pattern("{foo:word}", false, &mut types),
            Err(SubplotError::SimplePatternKindMismatch(_))
        ));
    }
}
