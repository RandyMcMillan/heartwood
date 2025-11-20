use std::collections::HashMap;

use textwrap::fill;

pub use crate::RoadmapError;
pub use crate::Status;
pub use crate::Step;

/// Error in Roadmap, from parsing or otherwise.
pub type RoadmapResult<T> = Result<T, RoadmapError>;

/// Represent a full project roadmap.
///
/// This stores all the steps needed to reach the end goal. See the
/// crate leve documentation for an example.
#[derive(Clone, Debug, Default)]
pub struct Roadmap {
    steps: Vec<Step>,
}

impl Roadmap {
    /// Create a new, empty roadmap.
    ///
    /// You probably want the `from_yaml` function instead.
    pub fn new(map: HashMap<String, Step>) -> Self {
        Self {
            steps: map.values().cloned().collect(),
        }
    }

    // Find steps that nothing depends on.
    fn goals(&self) -> Vec<&Step> {
        self.steps
            .iter()
            .filter(|step| self.is_goal(step))
            .collect()
    }

    /// Count number of steps that nothing depends on.
    pub fn count_goals(&self) -> usize {
        self.goals().len()
    }

    /// Iterate over step names.
    pub fn step_names(&self) -> impl Iterator<Item = &str> {
        self.steps.iter().map(|step| step.name())
    }

    /// Get a step, given its name.
    pub fn get_step(&self, name: &str) -> Option<&Step> {
        self.steps.iter().find(|step| step.name() == name)
    }

    /// Add a step to the roadmap.
    pub fn add_step(&mut self, step: Step) {
        self.steps.push(step);
    }

    // Get iterator over refs to steps.
    pub fn iter(&self) -> impl Iterator<Item = &Step> {
        self.steps.iter()
    }

    // Get iterator over mut refs to steps.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Step> {
        self.steps.iter_mut()
    }

    /// Compute status of any step for which it has not been specified
    /// in the input.
    pub fn set_missing_statuses(&mut self) {
        let new_steps: Vec<Step> = self
            .steps
            .iter()
            .map(|step| {
                let mut step = step.clone();
                if step.status() == Status::Unknown {
                    if self.is_goal(&step) {
                        step.set_status(Status::Goal);
                    } else if self.is_blocked(&step) {
                        step.set_status(Status::Blocked);
                    } else if self.is_ready(&step) {
                        step.set_status(Status::Ready);
                    }
                }
                step
            })
            .collect();

        if self.steps != new_steps {
            self.steps = new_steps;
            self.set_missing_statuses();
        }
    }

    /// Should unset status be ready? In other words, if there are any
    /// dependencies, they are all finished.
    pub fn is_ready(&self, step: &Step) -> bool {
        self.dep_statuses(step)
            .iter()
            .all(|&status| status == Status::Finished)
    }

    /// Should unset status be blocked? In other words, if there are
    /// any dependencies, that aren't finished.
    pub fn is_blocked(&self, step: &Step) -> bool {
        self.dep_statuses(step)
            .iter()
            .any(|&status| status != Status::Finished)
    }

    // Return vector of all statuses of all dependencies
    fn dep_statuses(&self, step: &Step) -> Vec<Status> {
        step.dependencies()
            .map(|depname| {
                if let Some(step) = self.get_step(depname) {
                    step.status()
                } else {
                    Status::Unknown
                }
            })
            .collect()
    }

    /// Should status be goal? In other words, does any other step
    /// depend on this one?
    pub fn is_goal(&self, step: &Step) -> bool {
        self.steps.iter().all(|other| !other.depends_on(step))
    }

    // Validate that the parsed, constructed roadmap is valid.
    pub fn validate(&self) -> RoadmapResult<()> {
        // Is there exactly one goal?
        let goals = self.goals();
        let n = goals.len();
        match n {
            0 => return Err(RoadmapError::NoGoals),
            1 => (),
            _ => {
                let names: Vec<String> = goals.iter().map(|s| s.name().into()).collect();
                return Err(RoadmapError::ManyGoals { count: n, names });
            }
        }

        // Does every dependency exist?
        for step in self.iter() {
            for depname in step.dependencies() {
                match self.get_step(depname) {
                    None => {
                        return Err(RoadmapError::MissingDep {
                            name: step.name().into(),
                            missing: depname.into(),
                        })
                    }
                    Some(_) => (),
                }
            }
        }

        Ok(())
    }

    /// Get a Graphviz dot language representation of a roadmap. This
    /// is the textual representation, and the caller needs to use the
    /// Graphviz dot(1) tool to create an image from it.
    pub fn format_as_dot(&self, label_width: usize) -> RoadmapResult<String> {
        self.validate()?;

        let labels = self.steps.iter().map(|step| {
            format!(
                "{} [label=\"{}\" style=filled fillcolor=\"{}\" shape=\"{}\"];\n",
                step.name(),
                fill(step.label(), label_width).replace('\n', "\\n"),
                Roadmap::get_status_color(step),
                Roadmap::get_status_shape(step),
            )
        });

        let mut dot = String::new();
        dot.push_str("digraph \"roadmap\" {\n");
        for line in labels {
            dot.push_str(&line);
        }

        for step in self.iter() {
            for dep in step.dependencies() {
                let line = format!("{} -> {};\n", dep, step.name());
                dot.push_str(&line);
            }
        }

        dot.push_str("}\n");

        Ok(dot)
    }

    fn get_status_color(step: &Step) -> &str {
        match step.status() {
            Status::Blocked => "#f4bada",
            Status::Finished => "#eeeeee",
            Status::Ready => "#ffffff",
            Status::Next => "#0cc00",
            Status::Goal => "#00eeee",
            Status::Unknown => "#ff0000",
        }
    }

    fn get_status_shape(step: &Step) -> &str {
        match step.status() {
            Status::Blocked => "rectangle",
            Status::Finished => "octagon",
            Status::Ready => "ellipse",
            Status::Next => "ellipse",
            Status::Goal => "diamond",
            Status::Unknown => "house",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Roadmap, Status, Step};
    use crate::from_yaml;

    #[test]
    fn new_roadmap() {
        let roadmap = Roadmap::default();
        assert_eq!(roadmap.step_names().count(), 0);
    }

    #[test]
    fn add_step_to_roadmap() {
        let mut roadmap = Roadmap::default();
        let first = Step::new("first", "the first step");
        roadmap.add_step(first);
        let names: Vec<&str> = roadmap.step_names().collect();
        assert_eq!(names, vec!["first"]);
    }

    #[test]
    fn get_step_from_roadmap() {
        let mut roadmap = Roadmap::default();
        let first = Step::new("first", "the first step");
        roadmap.add_step(first);
        let gotit = roadmap.get_step("first").unwrap();
        assert_eq!(gotit.name(), "first");
        assert_eq!(gotit.label(), "the first step");
    }

    #[test]
    fn set_missing_goal_status() {
        let mut r = from_yaml(
            "
goal:
  depends:
  - finished
  - blocked

finished:
  status: finished

ready:
  depends:
  - finished

next:
  status: next

blocked:
  depends:
  - ready
  - next
",
        )
        .unwrap();
        r.set_missing_statuses();
        assert_eq!(r.get_step("goal").unwrap().status(), Status::Goal);
        assert_eq!(r.get_step("finished").unwrap().status(), Status::Finished);
        assert_eq!(r.get_step("ready").unwrap().status(), Status::Ready);
        assert_eq!(r.get_step("next").unwrap().status(), Status::Next);
        assert_eq!(r.get_step("blocked").unwrap().status(), Status::Blocked);
    }

    #[test]
    fn empty_dot() {
        let roadmap = Roadmap::default();
        match roadmap.format_as_dot(999) {
            Err(_) => (),
            _ => panic!("expected error for empty roadmap"),
        }
    }

    #[test]
    fn simple_dot() {
        let mut roadmap = Roadmap::default();
        let mut first = Step::new("first", "");
        first.set_status(Status::Ready);
        let mut second = Step::new("second", "");
        second.add_dependency("first");
        second.set_status(Status::Goal);
        roadmap.add_step(first);
        roadmap.add_step(second);
        assert_eq!(
            roadmap.format_as_dot(999).unwrap(),
            "digraph \"roadmap\" {
first [label=\"\" style=filled fillcolor=\"#ffffff\" shape=\"ellipse\"];
second [label=\"\" style=filled fillcolor=\"#00eeee\" shape=\"diamond\"];
first -> second;
}
"
        );
    }
}
