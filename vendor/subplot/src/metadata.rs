use crate::{Bindings, SubplotError, TemplateSpec};

use lazy_static::lazy_static;
use log::trace;
use regex::Regex;
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Debug;
use std::ops::Deref;
use std::path::{Path, PathBuf};

lazy_static! {
    // Pattern that recognises a YAML block at the beginning of a file.
    static ref LEADING_YAML_PATTERN: Regex = Regex::new(r"^(?:\S*\n)*(?P<yaml>-{3,}\n([^.].*\n)*\.{3,}\n)(?P<text>(.*\n)*)$").unwrap();


    // Pattern that recognises a YAML block at the end of a file.
    static ref TRAILING_YAML_PATTERN: Regex = Regex::new(r"(?P<text>(.*\n)*)\n*(?P<yaml>-{3,}\n([^.].*\n)*\.{3,}\n)(?:\S*\n)*$").unwrap();
}

/// Errors from Markdown parsing.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Regex(#[from] regex::Error),

    #[error(transparent)]
    Yaml(#[from] marked_yaml::FromYamlError),
}

/// Document metadata.
///
/// This is expressed in the Markdown input file as an embedded YAML
/// block.
///
/// Note that this structure needs to be able to capture any metadata
/// block we can work with, in any input file. By being strict here we
/// make it easier to tell the user when a metadata block has, say, a
/// misspelled field.
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct YamlMetadata {
    title: String,
    subtitle: Option<String>,
    authors: Option<Vec<String>>,
    date: Option<String>,
    classes: Option<Vec<String>>,
    markdowns: Vec<PathBuf>,
    bindings: Option<Vec<PathBuf>>,
    documentclass: Option<String>,
    #[serde(default)]
    impls: BTreeMap<String, Vec<PathBuf>>,
    css_embed: Option<Vec<PathBuf>>,
    css_urls: Option<Vec<String>>,
}

impl YamlMetadata {
    #[cfg(test)]
    fn new(yaml_text: &str) -> Result<Self, Error> {
        let meta: Self = marked_yaml::from_yaml(0 /* TODO: Track sources */, yaml_text)?;
        Ok(meta)
    }

    /// Names of files with the Markdown for the subplot document.
    pub fn markdowns(&self) -> &[PathBuf] {
        &self.markdowns
    }

    /// Title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Subtitle.
    pub fn subtitle(&self) -> Option<&str> {
        self.subtitle.as_deref()
    }

    /// Date.
    pub fn date(&self) -> Option<&str> {
        self.date.as_deref()
    }

    /// Set date.
    pub fn set_date(&mut self, date: String) {
        self.date = Some(date);
    }

    /// Authors.
    pub fn authors(&self) -> Option<&[String]> {
        self.authors.as_deref()
    }

    /// Names of bindings files.
    pub fn bindings_filenames(&self) -> Option<&[PathBuf]> {
        self.bindings.as_deref()
    }

    /// Impls section.
    pub fn impls(&self) -> &BTreeMap<String, Vec<PathBuf>> {
        &self.impls
    }

    /// Classes..
    pub fn classes(&self) -> Option<&[String]> {
        self.classes.as_deref()
    }

    /// Documentclass.
    pub fn documentclass(&self) -> Option<&str> {
        self.documentclass.as_deref()
    }
}

#[cfg(test)]
mod test {
    use super::YamlMetadata;
    use std::path::{Path, PathBuf};

    #[test]
    fn full_meta() {
        let meta = YamlMetadata::new(
            "\
title: Foo Bar
date: today
classes: [json, text]
impls:
  python:
   - foo.py
   - bar.py
markdowns:
- test.md
bindings:
- foo.yaml
- bar.yaml
",
        )
        .unwrap();
        assert_eq!(meta.title, "Foo Bar");
        assert_eq!(meta.date.unwrap(), "today");
        assert_eq!(meta.classes.unwrap(), &["json", "text"]);
        assert_eq!(meta.markdowns, vec![Path::new("test.md")]);
        assert_eq!(
            meta.bindings.unwrap(),
            &[path("foo.yaml"), path("bar.yaml")]
        );
        assert!(!meta.impls.is_empty());
        for (k, v) in meta.impls.iter() {
            assert_eq!(k, "python");
            assert_eq!(v, &[path("foo.py"), path("bar.py")]);
        }
    }

    fn path(s: &str) -> PathBuf {
        PathBuf::from(s)
    }
}

/// Metadata of a document, as needed by Subplot.
#[derive(Debug)]
pub struct Metadata {
    basedir: PathBuf,
    title: String,
    date: Option<String>,
    authors: Option<Vec<String>>,
    markdown_filenames: Vec<PathBuf>,
    bindings_filenames: Vec<PathBuf>,
    bindings: Bindings,
    impls: HashMap<String, DocumentImpl>,
    /// Extra class names which should be considered 'correct' for this document
    classes: Vec<String>,
    css_embed: Vec<String>,
    css_urls: Vec<String>,
}

#[derive(Debug)]
pub struct DocumentImpl {
    spec: TemplateSpec,
    functions: Vec<PathBuf>,
}

impl Metadata {
    /// Create from YamlMetadata.
    pub fn from_yaml_metadata<P>(
        basedir: P,
        yaml: &YamlMetadata,
        template: Option<&str>,
    ) -> Result<Self, SubplotError>
    where
        P: AsRef<Path> + Debug,
    {
        let mut bindings = Bindings::new();
        let bindings_filenames = if let Some(filenames) = yaml.bindings_filenames() {
            get_bindings(filenames, &mut bindings, template)?;
            filenames.iter().map(|p| p.to_path_buf()).collect()
        } else {
            vec![]
        };

        let mut impls = HashMap::new();

        for (impl_name, functions_filenames) in yaml.impls().iter() {
            let template_spec = load_template_spec(impl_name)?;
            let filenames = pathbufs("", functions_filenames);
            let docimpl = DocumentImpl::new(template_spec, filenames);
            impls.insert(impl_name.to_string(), docimpl);
        }

        let classes = if let Some(v) = yaml.classes() {
            v.iter().map(|s| s.to_string()).collect()
        } else {
            vec![]
        };

        let mut css_embed = vec![];
        if let Some(filenames) = &yaml.css_embed {
            for filename in filenames.iter() {
                let css = std::fs::read(filename)
                    .map_err(|e| SubplotError::ReadFile(filename.into(), e))?;
                let css = String::from_utf8(css)
                    .map_err(|e| SubplotError::FileUtf8(filename.into(), e))?;
                css_embed.push(css);
            }
        }

        let css_urls = if let Some(urls) = &yaml.css_urls {
            urls.clone()
        } else {
            vec![]
        };

        let meta = Self {
            basedir: basedir.as_ref().to_path_buf(),
            title: yaml.title().into(),
            date: yaml.date().map(|s| s.into()),
            authors: yaml.authors().map(|a| a.into()),
            markdown_filenames: yaml.markdowns().into(),
            bindings_filenames,
            bindings,
            impls,
            classes,
            css_embed,
            css_urls,
        };
        trace!("metadata: {:#?}", meta);

        Ok(meta)
    }

    /// Return title of document.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Return date of document, if any.
    pub fn date(&self) -> Option<&str> {
        self.date.as_deref()
    }

    /// Set date.
    pub fn set_date(&mut self, date: String) {
        self.date = Some(date);
    }

    /// Authors.
    pub fn authors(&self) -> Option<&[String]> {
        self.authors.as_deref()
    }

    /// Return base dir for all relative filenames.
    pub fn basedir(&self) -> &Path {
        &self.basedir
    }

    /// Return filenames of the markdown files.
    pub fn markdown_filenames(&self) -> &[PathBuf] {
        &self.markdown_filenames
    }

    /// Return filename where bindings are specified.
    pub fn bindings_filenames(&self) -> Vec<&Path> {
        self.bindings_filenames.iter().map(|f| f.as_ref()).collect()
    }

    /// Return the document implementation (filenames, spec, etc) for the given template name
    pub fn document_impl(&self, template: &str) -> Option<&DocumentImpl> {
        self.impls.get(template)
    }

    /// Return the templates the document expects to implement
    pub fn templates(&self) -> impl Iterator<Item = &str> {
        self.impls.keys().map(String::as_str)
    }

    /// Return the bindings.
    pub fn bindings(&self) -> &Bindings {
        &self.bindings
    }

    /// The classes which this document also claims are valid
    pub fn classes(&self) -> impl Iterator<Item = &str> {
        self.classes.iter().map(Deref::deref)
    }

    /// Contents of CSS files to embed into the HTML output.
    pub fn css_embed(&self) -> impl Iterator<Item = &str> {
        self.css_embed.iter().map(Deref::deref)
    }

    /// List of CSS urls to add to the HTML output.
    pub fn css_urls(&self) -> impl Iterator<Item = &str> {
        self.css_urls.iter().map(Deref::deref)
    }
}

impl DocumentImpl {
    fn new(spec: TemplateSpec, functions: Vec<PathBuf>) -> Self {
        Self { spec, functions }
    }

    pub fn functions_filenames(&self) -> impl Iterator<Item = &Path> {
        self.functions.iter().map(PathBuf::as_path)
    }

    pub fn spec(&self) -> &TemplateSpec {
        &self.spec
    }
}

fn load_template_spec(template: &str) -> Result<TemplateSpec, SubplotError> {
    let mut spec_path = PathBuf::from(template);
    spec_path.push("template");
    spec_path.push("template.yaml");
    TemplateSpec::from_file(&spec_path)
}

fn pathbufs<P>(basedir: P, v: &[PathBuf]) -> Vec<PathBuf>
where
    P: AsRef<Path>,
{
    let basedir = basedir.as_ref();
    v.iter().map(|p| basedir.join(p)).collect()
}

fn get_bindings<P>(
    filenames: &[P],
    bindings: &mut Bindings,
    template: Option<&str>,
) -> Result<(), SubplotError>
where
    P: AsRef<Path> + Debug,
{
    for filename in filenames {
        bindings.add_from_file(filename, template)?;
    }
    Ok(())
}
