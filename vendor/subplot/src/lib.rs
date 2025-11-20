//! Process Subplot input files.
//!
//! Capture and communicate acceptance criteria for software and systems,
//! and how they are verified, in a way that's understood by all project
//! stakeholders.

#![deny(missing_docs)]

mod error;
pub use error::SubplotError;
pub use error::Warning;
pub use error::Warnings;

pub mod resource;

mod diagrams;
pub use diagrams::{DiagramMarkup, DotMarkup, MarkupOpts, PikchrMarkup, PlantumlMarkup, Svg};

mod embedded;
pub use embedded::EmbeddedFile;
pub use embedded::EmbeddedFiles;

mod policy;
pub use policy::get_basedir_from;

mod metadata;
pub use metadata::{Metadata, YamlMetadata};

mod doc;
pub mod html;
pub mod md;
pub mod mdparse;
pub use doc::Document;
pub use doc::{codegen, load_document, load_document_with_pullmark};
pub mod toc;

pub mod typeset;

mod style;
pub use style::Style;

mod scenarios;
pub use scenarios::Scenario;

mod steps;
pub use steps::{ScenarioStep, StepKind};

mod bindings;
pub use bindings::Binding;
pub use bindings::Bindings;

mod matches;
pub use matches::MatchedScenario;
pub use matches::MatchedStep;
pub use matches::MatchedSteps;
pub use matches::PartialStep;
pub use matches::StepSnippet;

mod templatespec;
pub use templatespec::TemplateSpec;

mod codegen;
pub use codegen::generate_test_program;
