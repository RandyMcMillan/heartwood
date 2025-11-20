use std::collections::HashMap;

pub use crate::Roadmap;
pub use crate::RoadmapResult;
pub use crate::Step;

/// Create a new roadmap from a textual YAML representation.
pub fn from_yaml(yaml: &str) -> RoadmapResult<Roadmap> {
    let mut roadmap: HashMap<String, Step> = marked_yaml::from_yaml(0, yaml)?;
    for (name, step) in roadmap.iter_mut() {
        step.set_name(name);
    }
    let roadmap = Roadmap::new(roadmap);
    roadmap.validate()?;
    Ok(roadmap)
}

#[cfg(test)]
mod tests {
    use super::from_yaml;

    #[test]
    fn yaml_is_empty() {
        assert!(from_yaml("").is_err());
    }

    #[test]
    fn yaml_is_list() {
        assert!(from_yaml("[]").is_err());
    }

    #[test]
    fn yaml_unknown_dep() {
        assert!(from_yaml("foo: {depends: [bar]}").is_err());
    }

    #[test]
    fn yaml_unknown_status() {
        assert!(from_yaml(r#"foo: {status: "bar"}"#).is_err());
    }

    #[test]
    fn yaml_happy() {
        let roadmap = from_yaml(
            "
first:
  label: the first step
second:
  label: the second step
  depends:
  - first
",
        )
        .unwrap();

        let names: Vec<&str> = roadmap.step_names().collect();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"first"));
        assert!(names.contains(&"second"));

        let first = roadmap.get_step("first").unwrap();
        assert_eq!(first.name(), "first");
        assert_eq!(first.label(), "the first step");
        let deps = first.dependencies().count();
        assert_eq!(deps, 0);

        let second = roadmap.get_step("second").unwrap();
        assert_eq!(second.name(), "second");
        assert_eq!(second.label(), "the second step");
        let deps: Vec<&str> = second.dependencies().collect();
        assert_eq!(deps, vec!["first"]);
    }
}
