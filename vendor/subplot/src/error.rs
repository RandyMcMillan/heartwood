use crate::html::{HtmlError, Location};
use crate::matches::MatchedSteps;
use crate::md::MdError;

use std::path::PathBuf;
use std::process::Output;

use thiserror::Error;

/// Define all the kinds of errors any part of this crate can return.
#[derive(Debug, Error)]
pub enum SubplotError {
    /// Scenario step does not start at the beginning of the line.
    #[error("Scenario step is indented: {0}")]
    NotAtBoln(String),

    /// Document has non-fatal errors.
    #[error("Document has {0} warnings.")]
    Warnings(usize),

    /// Subplot could not find its CSS file.
    #[error("failed to find CSS file: {0}")]
    CssFileNotFound(PathBuf, #[source] std::io::Error),

    /// Subplot could not find a file named as a bindings file.
    #[error("binding file could not be found: {0}")]
    BindingsFileNotFound(PathBuf, #[source] std::io::Error),

    /// Subplot could not find a file named as a functions file.
    #[error("functions file could not be found: {0}")]
    FunctionsFileNotFound(PathBuf, #[source] std::io::Error),

    /// The simple pattern specifies a kind that is unknown.
    #[error("simple pattern kind {0} is unknown")]
    UnknownSimplePatternKind(String),

    /// The simple pattern and the type map disagree over the kind of a match
    #[error("simple pattern kind for {0} disagrees with the type map")]
    SimplePatternKindMismatch(String),

    /// The simple pattern contains a stray { or }.
    #[error("simple pattern contains a stray {{ or }}")]
    StrayBraceInSimplePattern(String),

    /// The simple pattern has regex metacharacters.
    ///
    /// Simple patterns are not permitted to have regex metacharacters in them
    /// unless the pattern is explicitly marked `regex: false` to indicate that
    /// the binding author understands what they're up to.
    #[error("simple pattern contains regex metacharacters: {0}")]
    SimplePatternHasMetaCharacters(String),

    /// Error while parsing a bindings file
    #[error("binding file failed to parse: {0}")]
    BindingFileParseError(PathBuf, #[source] Box<SubplotError>),

    /// Binding lacks documentation.
    ///
    /// Add a `doc` field to the binding with text the documents the
    /// binding.
    #[error("binding lacks documentation: {0}: {1} {2}")]
    NoBindingDoc(PathBuf, crate::StepKind, String),

    /// Scenario step does not match a known binding
    ///
    /// This may be due to the binding missing entirely, or that the
    /// step or the binding has a typo, or that a pattern in the
    /// binding doesn't match what the author thinks it matches.
    #[error("do not understand binding: {0}")]
    BindingUnknown(String),

    /// Scenario step matches more than one binding
    ///
    /// THis may be due to bindings being too general, or having unusual
    /// overlaps in their matching
    #[error("more than one binding matches step {0}:\n{1}")]
    BindingNotUnique(String, MatchedSteps),

    /// A binding in the bindings file doesn't specify a known keyword.
    #[error("binding doesn't specify known keyword: {0}")]
    BindingWithoutKnownKeyword(String),

    /// A binding has more than one keyword (given/when/then).
    #[error("binding has more than one keyword (given/when/then)")]
    BindingHasManyKeywords(String),

    /// A binding lists an unknown type in its type map
    #[error("binding has unknown type/kind {0}")]
    UnknownTypeInBinding(String),

    /// Subplot tried to use a program, but couldn't feed it data
    ///
    /// Subplot uses some helper programs to implement some of its
    /// functionality, for example the GraphViz dot program. This
    /// error means that when tried to start a helper program and
    /// write to the helper's standard input stream, it failed to get
    /// the stream.
    ///
    /// This probably implies there's something wrong on your system.
    #[error("couldn't get stdin of child process to write to it")]
    ChildNoStdin,

    /// Subplot helper program failed
    ///
    /// Subplot uses some helper programs to implement some of its
    /// functionality, for example the GraphViz dot program. This
    /// error means that the helper program failed (exit code was not
    /// zero).
    ///
    /// This probably implies there's something wrong in Subplot.
    /// Please report this error to the Subplot project.
    #[error("child process failed: {0}")]
    ChildFailed(String),

    /// Binding doesn't define a function
    ///
    /// All binding must define the name of the function that
    /// implements the step. The bindings file has at least one
    /// binding that doesn't define one. To fix, add a `function:`
    /// field to the binding.
    #[error("binding does not name a function: {0}")]
    NoFunction(String),

    /// Document has no title
    ///
    /// The document YAML metadata does not define a document title.
    /// To fix, add a `title` field.
    #[error("document has no title")]
    NoTitle,

    /// Document has no template
    ///
    /// The document YAML metadata does not define the template to use
    /// during code generation.
    ///
    /// To fix, ensure an appropriate `impl` entry is present.
    #[error("document has no template")]
    MissingTemplate,

    /// Document has more than one template
    ///
    /// The document YAML metadata specifies more than one possible
    /// template implementation to be used during code generation.
    ///
    /// To fix, specify `--template` on the codegen CLI.
    #[error("document has more than one template possibility")]
    AmbiguousTemplate,

    /// Document does not support the requested template
    ///
    /// The document YAML metadata does not specify support for the
    /// stated template.
    ///
    /// To fix, specify a template which is provided for in the document.
    #[error("document lacks specified template support")]
    TemplateSupportNotPresent,

    /// First scenario is before first heading
    ///
    /// Subplot scenarios are group by the input document's structure.
    /// Each scenario must be in a chapter, section, subsection, or
    /// other structural element with a heading. Subplot found a
    /// scenario block before the first heading in the document.
    ///
    /// To fix, add a heading or move the scenario after a heading.
    #[error("{0}: first scenario is before first heading")]
    ScenarioBeforeHeading(Location),

    /// Step does not have a keyword.
    #[error("step has no keyword: {0}")]
    NoStepKeyword(String),

    /// Unknown scenario step keyword.
    ///
    /// Each scenario step must start with a known keyword (given,
    /// when, then, and, but), but Subplot didn't find one it
    /// recognized.
    ///
    /// This is usually due to a typing mistake or similar.
    #[error("unknown step keyword: {0}")]
    UnknownStepKind(String),

    /// Scenario step uses continuation keyword too early
    ///
    /// If a continuation keyword (`and` or `but`) is used too early
    /// in a scenario (i.e. before any other keyword was used) then
    /// it cannot be resolved to whichever keyword it should have been.
    #[error("continuation keyword used too early")]
    ContinuationTooEarly,

    /// Scenario has the same title as another scenario
    ///
    /// Titles of scenarios must be unique in the input document,
    /// but Subplot found at least one with the same title as another.
    #[error("Scenario title is duplicate: {0:?}")]
    DuplicateScenario(String),

    /// Embedded file has the same name as another embedded file
    ///
    /// Names of embedded files must be unique in the input document,
    /// but Subplot found at least one with the same name as another.
    #[error("Duplicate embedded file name: {0:?}")]
    DuplicateEmbeddedFilename(String),

    /// Embedded file has more than one `add-newline` attribute
    ///
    /// The `add-newline` attribute can only be specified once for any given
    /// embedded file
    #[error("Embedded file {0} has more than one `add-newline` attribute")]
    RepeatedAddNewlineAttribute(String),

    /// Unrecognised `add-newline` attribute value on an embedded file
    ///
    /// The `add-newline` attribute can only take the values `auto`, `yes`,
    /// and `no`.
    #[error("Embedded file {0} has unrecognised `add-newline={1}` - valid values are auto/yes/no")]
    UnrecognisedAddNewline(String, String),

    /// Couldn't determine base directory from input file name.
    ///
    /// Subplot needs to to determine the base directory for files
    /// referred to by the markdown input file (e.g., bindings and
    /// functions files). It failed to do that from the name of the
    /// input file. Something weird is happening.
    #[error("Could not determine base directory for included files from {0:?}")]
    BasedirError(PathBuf),

    /// Output goes into a directory that does not exist.
    ///
    /// Subplot needs to know in which directory it should write its
    /// output file, since it writes a temporary file first, then
    /// renames it to the final output file. The temporary file is
    /// created in the same directory as the final output file.
    /// However, Subplot could not find that directory.
    #[error("Output going to a directory that does not exist: {0}")]
    OutputDirectoryNotFound(String),

    /// The template.yaml is not in a directory.
    ///
    /// Template specifications reference files relative to the
    /// template.yaml file, but Subplot could not find the name of the
    /// directory containing the template.yaml file. Something is very
    /// weird.
    #[error("Couldn't find name of directory containing template spec: {0}")]
    NoTemplateSpecDirectory(PathBuf),

    /// A code template has an error.
    #[error("Couldn't load template {0}")]
    TemplateError(String, #[source] tera::Error),

    /// Unknown classes in use in document
    #[error("Unknown classes found in the document: {0}")]
    UnknownClasses(String),

    /// Template does not specify how to run generated program
    ///
    /// The template.yaml file used does not specify how to run the
    /// generated program, but user asked codegen to run it.
    #[error("template.yaml does not specify how to run generated program")]
    TemplateNoRun,

    /// An embedded file was not found.
    #[error("embedded file {0} was not found in the subplot document")]
    EmbeddedFileNotFound(String),

    /// When rendering a pikchr, something went wrong.
    #[error("failure rendering pikchr diagram: {0}")]
    PikchrRenderError(String),

    /// When attempting to codegen, no scenarios matched the desired template language
    #[error("no scenarios were found matching the `{0}` template")]
    NoScenariosMatched(String),

    /// Failed to invoke a program.
    #[error("Failed to invoke {0}")]
    Spawn(PathBuf, #[source] std::io::Error),

    /// Failed to write to stdin of child process.
    #[error("Failed to write to stdin of child process")]
    WriteToChild(#[source] std::io::Error),

    /// Error when waiting for child process to finish.
    #[error("Error when waiting for child process to finish")]
    WaitForChild(#[source] std::io::Error),

    /// Error when reading a file.
    #[error("Error when reading {0}")]
    ReadFile(PathBuf, #[source] std::io::Error),

    /// Error when creating a file.
    #[error("Error when creating {0}")]
    CreateFile(PathBuf, #[source] std::io::Error),

    /// Error when writing to a file.
    #[error("Error when writing to {0}")]
    WriteFile(PathBuf, #[source] std::io::Error),

    /// Error parsing markdown into HTML.
    #[error(transparent)]
    ParseMarkdown(#[from] HtmlError),

    /// Regular expression error
    ///
    /// Subplot uses regular expressions. This is a generic wrapper for
    /// any kinds of errors related to that.
    #[error("Failed to compile regular expression: {0:?}")]
    Regex(String, #[source] regex::Error),

    /// Error parsing YAML metadata for document.
    #[error("Failed to parse YAML metadata")]
    Metadata(#[source] marked_yaml::FromYamlError),

    /// Error parsing YAML metadata for document, from external file.
    #[error("Failed to parse YAML metadata in {0}")]
    MetadataFile(PathBuf, #[source] marked_yaml::FromYamlError),

    /// UTF8 conversion error.
    #[error("failed to parse UTF8 in file {0}")]
    FileUtf8(PathBuf, #[source] std::string::FromUtf8Error),

    /// UTF8 conversion error.
    #[error(transparent)]
    Utf8Error(#[from] std::str::Utf8Error),

    /// Markdown errors.
    #[error(transparent)]
    MdError(#[from] MdError),

    /// String formatting failed.
    #[error("Failed in string formattiing: {0}")]
    StringFormat(std::fmt::Error),

    /// Input file could not be read.
    #[error("Failed to read input file {0}")]
    InputFileUnreadable(PathBuf, #[source] std::io::Error),

    /// Input file mtime lookup.
    #[error("Failed to get modification time of {0}")]
    InputFileMtime(PathBuf, #[source] std::io::Error),

    /// Error typesetting a roadmap diagram.
    #[error(transparent)]
    Roadmap(#[from] roadmap::RoadmapError),
}

impl SubplotError {
    /// Construct a ChildFailed error.
    pub fn child_failed(msg: &str, output: &Output) -> SubplotError {
        let msg = format!(
            "{}: {}: {:?}",
            msg,
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stderr)
        );
        SubplotError::ChildFailed(msg)
    }
}

/// A warning, or non-fatal error.
///
/// Errors prevent Subplot from producing output. Warnings don't do that.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Warning {
    /// Document refers to an embedded file that doesn't exist.
    #[error(
        "Document refers to an embedded file that doesn't exist: \"{1}\"\n  in scenario \"{0}\""
    )]
    UnknownEmbeddedFile(String, String),

    /// Embedded file is not used by any scenario.
    #[error("Embedded file is not used by any scenario: \"{0}\"")]
    UnusedEmbeddedFile(String),

    /// Missing step implementation.
    #[error("Missing step implementation: \"{1}\"\n  in scenario \"{0}\"")]
    MissingStepImplementation(String, String),

    /// Unknown binding when typesetting a scenario.
    #[error("Unknown binding: {0}")]
    UnknownBinding(String),

    /// Pikchr failed during typesetting.
    #[error("Markup using pikchr failed: {0}")]
    Pikchr(String),

    /// Dot failed during typesetting.
    #[error("Markup using dot failed: {0}")]
    Dot(String),

    /// Plantuml failed during typesetting.
    #[error("Markup using plantuml failed: {0}")]
    Plantuml(String),

    /// A code block has an identifier but is not marked as a file or example
    #[error("Code block has identifier but lacks file or example class.  Is this a mistake? #{0} at {1}")]
    MissingAppropriateClassOnNamedCodeBlock(String, String),

    /// A capture in a binding is missing a name
    #[error("{0}: {1} - missing a name for the {2} capture")]
    MissingCaptureName(PathBuf, String, String),

    /// A capture in a binding is missing a type
    #[error("{0}: {1} - missing a type for the capture called {2}")]
    MissingCaptureType(PathBuf, String, String),
}

/// A list of warnings.
///
/// Subplot collects warnings into this structure so that they can be
/// processed at the end.
#[derive(Debug, Default)]
pub struct Warnings {
    warnings: Vec<Warning>,
}

impl Warnings {
    /// Append a warning to the list.
    pub fn push(&mut self, w: Warning) {
        self.warnings.push(w);
    }

    /// Append all warnings from one list to another.
    pub fn push_all(&mut self, mut other: Warnings) {
        self.warnings.append(&mut other.warnings);
    }

    /// Return a slice with all the warnings in the list.
    pub fn warnings(&self) -> &[Warning] {
        &self.warnings
    }

    /// Is the underlying warning set empty?
    pub fn is_empty(&self) -> bool {
        self.warnings.is_empty()
    }

    /// The number of warninings
    pub fn len(&self) -> usize {
        self.warnings.len()
    }
}
