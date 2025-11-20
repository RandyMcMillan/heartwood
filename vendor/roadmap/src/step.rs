use super::Status;
use serde::Deserialize;
use std::fmt;

/// A roadmap step.
///
/// See the crate documentation for an example. You
/// probably don't want to create steps manually, but via the roadmap
/// YAML parsing function.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    #[serde(skip)]
    name: String,
    #[serde(default)]
    label: String,
    #[serde(default)]
    status: Status,
    #[serde(default)]
    depends: Vec<String>,
}

impl Step {
    /// Create a new step with a name and a label.
    pub fn new(name: &str, label: &str) -> Step {
        Step {
            name: name.to_string(),
            status: Status::Unknown,
            label: label.to_string(),
            depends: vec![],
        }
    }

    /// Set the name of a step.
    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    /// Return the name of a step.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the label of a step.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Return the status of a step.
    pub fn status(&self) -> Status {
        self.status
    }

    /// Set the status of a step.
    pub fn set_status(&mut self, status: Status) {
        self.status = status
    }

    /// Return vector of names of dependencies for a step.
    pub fn dependencies(&self) -> impl Iterator<Item = &str> {
        self.depends.iter().map(|s| s.as_str())
    }

    /// Add the name of a dependency to step. Steps are referred by
    /// name. Steps don't know about other steps, and can't validate
    /// that the dependency exists, so this always works.
    pub fn add_dependency(&mut self, name: &str) {
        self.depends.push(name.to_owned());
    }

    /// Does this step depend on given other step?
    pub fn depends_on(&self, other: &Step) -> bool {
        self.depends.iter().any(|depname| depname == other.name())
    }
}

impl fmt::Display for Step {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::{Status, Step};

    #[test]
    fn new_step() {
        let step = Step::new("myname", "my label");
        assert_eq!(step.name(), "myname");
        assert_eq!(step.status(), Status::Unknown);
        assert_eq!(step.label(), "my label");
        assert_eq!(step.dependencies().count(), 0);
    }

    #[test]
    fn set_status() {
        let mut step = Step::new("myname", "my label");
        step.set_status(Status::Next);
        assert_eq!(step.status(), Status::Next);
    }

    #[test]
    fn add_step_dependency() {
        let mut second = Step::new("second", "the second step");
        second.add_dependency("first");
        let deps: Vec<_> = second.dependencies().collect();
        assert_eq!(deps, vec!["first"]);
    }
}
