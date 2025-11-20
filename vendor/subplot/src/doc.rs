use crate::bindings::CaptureType;
use crate::generate_test_program;
use crate::get_basedir_from;
use crate::md::Markdown;
use crate::toc::TableOfContents;
use crate::typeset::typeset_doc;
use crate::EmbeddedFile;
use crate::EmbeddedFiles;
use crate::MatchedScenario;
use crate::PartialStep;
use crate::Scenario;
use crate::Style;
use crate::SubplotError;
use crate::{Metadata, YamlMetadata};
use crate::{Warning, Warnings};

use std::collections::HashSet;
use std::default::Default;
use std::fmt::Debug;
use std::fs::read_to_string;
use std::ops::Deref;
use std::path::{Path, PathBuf};

use log::{error, trace};

/// The set of known (special) classes which subplot will always recognise
/// as being valid.
static SPECIAL_CLASSES: &[&str] = &[
    "scenario", "file", "example", "dot", "pikchr", "plantuml", "roadmap",
];

/// The set of known (file-type) classes which subplot will always recognise
/// as being valid.
static KNOWN_FILE_CLASSES: &[&str] = &["rust", "yaml", "python", "sh", "shell", "markdown", "bash"];

/// The set of known classes which subplot will always recognise as
/// being valid. We include the subplot-specific noNumberLines class
/// which we use to invert the default numberLines on .file blocks.
static KNOWN_BLOCK_CLASSES: &[&str] = &["numberLines", "noNumberLines"];

/// The set of classes which subplot will recognise as being appropriate
/// for having IDs.
static ID_OK_CLASSES: &[&str] = &["file", "example"];

/// A parsed Subplot document.
///
/// # Example
///
/// fix this example;
/// ~~~~ignored
/// let markdown = "\
/// ---
/// title: Test Title
/// ...
/// This is a test document.
/// ";
///
/// use std::io::Write;
/// use tempfile::NamedTempFile;
/// let mut f = NamedTempFile::new().unwrap();
/// f.write_all(markdown.as_bytes()).unwrap();
/// let filename = f.path();
///
/// use subplot;
/// let basedir = std::path::Path::new(".");
/// let style = subplot::Style::default();
/// let doc = subplot::Document::from_file(&basedir, filename, style, None).unwrap();
/// assert_eq!(doc.files(), &[]);
/// ~~~~
#[derive(Debug)]
pub struct Document {
    subplot: PathBuf,
    markdowns: Vec<Markdown>,
    meta: Metadata,
    toc: TableOfContents,
    files: EmbeddedFiles,
    style: Style,
}

impl Document {
    fn new(
        subplot: PathBuf,
        markdowns: Vec<Markdown>,
        meta: Metadata,
        toc: TableOfContents,
        files: EmbeddedFiles,
        style: Style,
    ) -> Document {
        let doc = Document {
            subplot,
            markdowns,
            meta,
            toc,
            files,
            style,
        };
        trace!("Document::new -> {:#?}", doc);
        doc
    }

    fn all_files(markdowns: &[Markdown]) -> Result<EmbeddedFiles, SubplotError> {
        let mut files = EmbeddedFiles::default();
        for md in markdowns {
            for file in md.embedded_files()?.files() {
                files.push(file.clone());
            }
        }
        Ok(files)
    }

    /// Construct a Document from a named file.
    pub fn from_file(
        basedir: &Path,
        filename: &Path,
        style: Style,
        template: Option<&str>,
    ) -> Result<Document, SubplotError> {
        trace!(
            "Document::from_file: basedir={} filename={}",
            basedir.display(),
            filename.display()
        );

        let meta = load_metadata_from_yaml_file(filename)?;
        trace!("metadata from YAML file: {:#?}", meta);

        let mut toc = TableOfContents::default();
        let mut markdowns = vec![];
        for filename in meta.markdowns() {
            let filename = basedir.join(filename);
            markdowns.push(Markdown::load_file(&filename, &mut toc)?);
        }

        let meta = Metadata::from_yaml_metadata(basedir, &meta, template)?;
        trace!("metadata from YAML: {:#?}", meta);
        let files = Self::all_files(&markdowns)?;
        let doc = Document::new(filename.into(), markdowns, meta, toc, files, style);

        trace!("Loaded document OK");
        Ok(doc)
    }

    /// List of markdown files.
    pub fn markdowns(&self) -> &[Markdown] {
        &self.markdowns
    }

    /// Return table of contents.
    pub fn toc(&self) -> &TableOfContents {
        &self.toc
    }

    /// Return Document as an HTML page serialized into HTML text
    pub fn to_html(&mut self, date: &str) -> Result<String, SubplotError> {
        self.meta.set_date(date.into());
        typeset_doc(self)
    }

    /// Return the document's metadata.
    pub fn meta(&self) -> &Metadata {
        &self.meta
    }

    /// Set document date.
    pub fn set_date(&mut self, date: String) {
        self.meta.set_date(date);
    }

    /// Return all source filenames for the document.
    ///
    /// The sources are any files that affect the output so that if
    /// the source file changes, the output needs to be re-generated.
    pub fn sources(&self, template: Option<&str>) -> Vec<PathBuf> {
        let mut names = vec![self.subplot.clone()];

        for x in self.meta().bindings_filenames() {
            names.push(PathBuf::from(x))
        }

        if let Some(template) = template {
            if let Some(spec) = self.meta().document_impl(template) {
                for x in spec.functions_filenames() {
                    names.push(PathBuf::from(x));
                }
            }
        } else {
            for template in self.meta().templates() {
                if let Some(spec) = self.meta().document_impl(template) {
                    for x in spec.functions_filenames() {
                        names.push(PathBuf::from(x));
                    }
                }
            }
        }

        for name in self.meta().markdown_filenames() {
            names.push(name.into());
        }

        for md in self.markdowns.iter() {
            let mut images = md.images();
            names.append(&mut images);
        }

        names
    }

    /// Return list of files embeddedin the document.
    pub fn embedded_files(&self) -> &[EmbeddedFile] {
        self.files.files()
    }

    /// Check the document for common problems.
    pub fn lint(&self) -> Result<(), SubplotError> {
        trace!("Linting document");
        self.check_doc_has_title()?;
        self.check_scenarios_are_unique()?;
        self.check_filenames_are_unique()?;
        self.check_block_classes()?;
        trace!("No linting problems found");
        Ok(())
    }

    // Check that all titles for scenarios are unique.
    fn check_scenarios_are_unique(&self) -> Result<(), SubplotError> {
        let mut known = HashSet::new();
        for title in self.scenarios()?.iter().map(|s| s.title().to_lowercase()) {
            if known.contains(&title) {
                return Err(SubplotError::DuplicateScenario(title));
            }
            known.insert(title);
        }
        Ok(())
    }

    // Check that all filenames for embedded files are unique.
    fn check_filenames_are_unique(&self) -> Result<(), SubplotError> {
        let mut known = HashSet::new();
        for filename in self
            .embedded_files()
            .iter()
            .map(|f| f.filename().to_lowercase())
        {
            if known.contains(&filename) {
                return Err(SubplotError::DuplicateEmbeddedFilename(filename));
            }
            known.insert(filename);
        }
        Ok(())
    }

    // Check that document has a title in its metadata.
    fn check_doc_has_title(&self) -> Result<(), SubplotError> {
        if self.meta().title().is_empty() {
            Err(SubplotError::NoTitle)
        } else {
            Ok(())
        }
    }

    /// Check that all the block classes in the document are known
    fn check_block_classes(&self) -> Result<(), SubplotError> {
        let classes_in_doc = self.all_block_classes();

        // Build the set of known good classes
        let mut known_classes: HashSet<String> = HashSet::new();
        for class in std::iter::empty()
            .chain(SPECIAL_CLASSES.iter().map(Deref::deref))
            .chain(KNOWN_FILE_CLASSES.iter().map(Deref::deref))
            .chain(KNOWN_BLOCK_CLASSES.iter().map(Deref::deref))
            .chain(self.meta().classes())
        {
            known_classes.insert(class.to_string());
        }
        // Acquire the set of used names which are not known
        let unknown_classes: Vec<_> = classes_in_doc.difference(&known_classes).cloned().collect();
        // If the unknown classes list is not empty, we had a problem and
        // we will report it to the user.
        if !unknown_classes.is_empty() {
            Err(SubplotError::UnknownClasses(unknown_classes.join(", ")))
        } else {
            Ok(())
        }
    }

    fn all_block_classes(&self) -> HashSet<String> {
        let mut set = HashSet::new();
        for md in self.markdowns.iter() {
            for class in md.block_classes() {
                set.insert(class);
            }
        }
        set
    }

    /// Check bindings for warnings
    pub fn check_bindings(&self, warnings: &mut Warnings) -> Result<(), SubplotError> {
        self.meta.bindings().check(warnings)
    }

    /// Check labelled code blocks have some appropriate class
    pub fn check_named_code_blocks_have_appropriate_class(
        &self,
        warnings: &mut Warnings,
    ) -> Result<bool, SubplotError> {
        let mut okay = true;
        for md in self.markdowns.iter() {
            for block in md.named_blocks() {
                if !block.all_attrs().iter().any(|attr| {
                    attr.name() == "class"
                        && ID_OK_CLASSES
                            .iter()
                            .any(|class| attr.value() == Some(class))
                }) {
                    // For now, named blocks must be files
                    warnings.push(Warning::MissingAppropriateClassOnNamedCodeBlock(
                        block
                            .attr("id")
                            .expect("Named blocks should have IDs")
                            .value()
                            .unwrap_or("(unknown-id)")
                            .to_string(),
                        block.location().to_string(),
                    ));
                    okay = false;
                }
            }
        }
        Ok(okay)
    }

    /// Check that all named files (in matched steps) are actually present in the
    /// document.
    pub fn check_named_files_exist(
        &self,
        template: &str,
        warnings: &mut Warnings,
    ) -> Result<bool, SubplotError> {
        let filenames: HashSet<_> = self
            .embedded_files()
            .iter()
            .map(|f| f.filename().to_lowercase())
            .collect();
        trace!("Checking that files exist");
        let mut okay = true;
        let scenarios = match self.matched_scenarios(template) {
            Ok(scenarios) => scenarios,
            Err(_) => return Ok(true), // We can't do the check, so say it's okay.
        };
        for scenario in scenarios {
            for step in scenario.steps() {
                for captured in step.parts() {
                    if let PartialStep::CapturedText {
                        name,
                        text,
                        kind: _,
                    } = captured
                    {
                        if matches!(step.types().get(name.as_str()), Some(CaptureType::File))
                            && !filenames.contains(&text.to_lowercase())
                        {
                            warnings.push(Warning::UnknownEmbeddedFile(
                                scenario.title().to_string(),
                                text.to_string(),
                            ));
                            okay = false;
                        }
                    }
                }
            }
        }
        Ok(okay)
    }

    /// Check that all embedded files are used by matched steps.
    pub fn check_embedded_files_are_used(
        &self,
        template: &str,
        warnings: &mut Warnings,
    ) -> Result<bool, SubplotError> {
        let mut filenames: HashSet<_> = self
            .embedded_files()
            .iter()
            .map(|f| f.filename().to_lowercase())
            .collect();
        trace!("Checking that files are used");
        let scenarios = match self.matched_scenarios(template) {
            Ok(scenarios) => scenarios,
            Err(_) => return Ok(true), // We can't do the check, so say it's okay.
        };
        for scenario in scenarios {
            for step in scenario.steps() {
                for captured in step.parts() {
                    if let PartialStep::CapturedText {
                        name,
                        text,
                        kind: _,
                    } = captured
                    {
                        if matches!(step.types().get(name.as_str()), Some(CaptureType::File)) {
                            filenames.remove(&text.to_lowercase());
                        }
                    }
                }
            }
        }
        for filename in filenames.iter() {
            warnings.push(Warning::UnusedEmbeddedFile(filename.to_string()));
        }

        // We always succeed. Subplot's own subplot had valid cases of
        // an embedded file being used and we need to develop a way to
        // mark such uses as OK, before we can make it an error to not
        // use an embedded file in a scenario.
        Ok(true)
    }

    /// Check that all matched steps actually have function implementations
    pub fn check_matched_steps_have_impl(&self, template: &str, warnings: &mut Warnings) -> bool {
        trace!("Checking that steps have implementations");
        let mut okay = true;
        let scenarios = match self.matched_scenarios(template) {
            Ok(scenarios) => scenarios,
            Err(_) => return true, // No matches means no missing impls
        };
        trace!("Found {} scenarios", scenarios.len());
        for scenario in scenarios {
            trace!("Checking that steps in scenario");
            for step in scenario.steps() {
                if step.function().is_none() {
                    trace!("Missing step implementation: {:?}", step.text());
                    warnings.push(Warning::MissingStepImplementation(
                        scenario.title().to_string(),
                        step.text().to_string(),
                    ));
                    okay = false;
                }
            }
        }
        okay
    }

    /// Typeset a Subplot document.
    pub fn typeset(
        &mut self,
        warnings: &mut Warnings,
        template: Option<&str>,
    ) -> Result<(), SubplotError> {
        for md in self.markdowns.iter_mut() {
            warnings.push_all(md.typeset(self.style.clone(), template, self.meta.bindings()));
        }
        Ok(())
    }

    /// Return all scenarios in a document.
    pub fn scenarios(&self) -> Result<Vec<Scenario>, SubplotError> {
        let mut scenarios = vec![];
        for md in self.markdowns.iter() {
            scenarios.append(&mut md.scenarios()?);
        }
        Ok(scenarios)
    }

    /// Return matched scenarios in a document.
    pub fn matched_scenarios(&self, template: &str) -> Result<Vec<MatchedScenario>, SubplotError> {
        let scenarios = self.scenarios()?;
        trace!(
            "Found {} scenarios, checking their bindings",
            scenarios.len()
        );
        let bindings = self.meta().bindings();
        scenarios
            .iter()
            .map(|scen| MatchedScenario::new(template, scen, bindings))
            .collect()
    }

    /// Extract a template name from this document
    pub fn template(&self) -> Result<&str, SubplotError> {
        let templates: Vec<_> = self.meta().templates().collect();
        if templates.len() == 1 {
            Ok(templates[0])
        } else if templates.is_empty() {
            Err(SubplotError::MissingTemplate)
        } else {
            Err(SubplotError::AmbiguousTemplate)
        }
    }
}

fn load_metadata_from_yaml_file(filename: &Path) -> Result<YamlMetadata, SubplotError> {
    let yaml = read_to_string(filename).map_err(|e| SubplotError::ReadFile(filename.into(), e))?;
    trace!("Parsing YAML metadata from {}", filename.display());
    let meta: YamlMetadata =
        marked_yaml::from_yaml(0 /* TODO: Keep track of sources */, &yaml)
            .map_err(|e| SubplotError::MetadataFile(filename.into(), e))?;
    Ok(meta)
}

/// Load a `Document` from a file.
pub fn load_document<P>(
    filename: P,
    style: Style,
    template: Option<&str>,
) -> Result<Document, SubplotError>
where
    P: AsRef<Path> + Debug,
{
    let filename = filename.as_ref();
    let base_path = get_basedir_from(filename);
    trace!(
        "Loading document based at `{}` called `{}` with {:?}",
        base_path.display(),
        filename.display(),
        style
    );
    let doc = Document::from_file(&base_path, filename, style, template)?;
    trace!("Loaded doc from file OK");

    Ok(doc)
}

/// Load a `Document` from a file.
///
/// This version uses the `cmark-pullmark` crate to parse Markdown.
pub fn load_document_with_pullmark<P>(
    filename: P,
    style: Style,
    template: Option<&str>,
) -> Result<Document, SubplotError>
where
    P: AsRef<Path> + Debug,
{
    let filename = filename.as_ref();
    let base_path = get_basedir_from(filename);
    trace!(
        "Loading document based at `{}` called `{}` with {:?} using pullmark-cmark",
        base_path.display(),
        filename.display(),
        style
    );
    crate::resource::add_search_path(filename.parent().unwrap());
    let doc = Document::from_file(&base_path, filename, style, template)?;
    trace!("Loaded doc from file OK");
    Ok(doc)
}

/// Generate code for one document.
pub fn codegen(
    filename: &Path,
    output: &Path,
    template: Option<&str>,
) -> Result<CodegenOutput, SubplotError> {
    let r = load_document_with_pullmark(filename, Style::default(), template);
    let mut doc = match r {
        Ok(doc) => doc,
        Err(err) => {
            return Err(err);
        }
    };
    doc.lint()?;
    let template = template
        .map(Ok)
        .unwrap_or_else(|| doc.template())?
        .to_string();
    trace!("Template: {:?}", template);
    if !doc.meta().templates().any(|t| t == template) {
        return Err(SubplotError::TemplateSupportNotPresent);
    }
    let mut warnings = Warnings::default();
    if !doc.check_named_code_blocks_have_appropriate_class(&mut warnings)?
        || !doc.check_named_files_exist(&template, &mut warnings)?
        || !doc.check_matched_steps_have_impl(&template, &mut warnings)
        || !doc.check_embedded_files_are_used(&template, &mut warnings)?
    {
        error!("Found problems in document, cannot continue");
        std::process::exit(1);
    }

    trace!("Generating code");

    generate_test_program(&mut doc, output, &template)?;
    trace!("Finished generating code");

    Ok(CodegenOutput::new(template, doc))
}

pub struct CodegenOutput {
    pub template: String,
    pub doc: Document,
}

impl CodegenOutput {
    fn new(template: String, doc: Document) -> Self {
        Self { template, doc }
    }
}
