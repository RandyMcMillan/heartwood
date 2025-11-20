use thiserror::Error;

/// Errors that can be returned for roadmaps.
#[derive(Error, Debug)]
pub enum RoadmapError {
    #[error("roadmap has no goals, must have exactly one")]
    NoGoals,

    #[error("too many goals, must have exactly one: found {count:}: {}", .names.join(", "))]
    ManyGoals { count: usize, names: Vec<String> },

    #[error("step {name:} depends on missing {missing:}")]
    MissingDep { name: String, missing: String },

    #[error("step is not a mapping")]
    StepNotMapping,

    #[error("'depends' must be a list of step names")]
    DependsNotNames,

    #[error("unknown status: {0}")]
    UnknownStatus(String),

    #[error(transparent)]
    SerdeError(#[from] marked_yaml::FromYamlError),
}
