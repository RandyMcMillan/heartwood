//! A parsed Markdown document.

use crate::{
    html::{Attribute, Content, Element, ElementTag, Location},
    mdparse::parse,
    steps::parse_scenario_snippet,
    toc::TableOfContents,
    typeset::typeset_element,
    Bindings, EmbeddedFile, EmbeddedFiles, Scenario, Style, SubplotError, Warnings,
};
use log::trace;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// A parsed Markdown document.
#[derive(Debug)]
pub struct Markdown {
    html: Element,
}

impl Markdown {
    /// Load a Markdown file.
    pub fn load_file(filename: &Path, toc: &mut TableOfContents) -> Result<Self, SubplotError> {
        trace!("parsing file as markdown: {}", filename.display());
        let text = std::fs::read(filename)
            .map_err(|e| SubplotError::InputFileUnreadable(filename.into(), e))?;
        let text = std::str::from_utf8(&text).map_err(SubplotError::Utf8Error)?;
        Self::new_from_str(filename, text, toc)
    }

    fn new_from_str(
        filename: &Path,
        text: &str,
        toc: &mut TableOfContents,
    ) -> Result<Self, SubplotError> {
        let html = parse(filename, text, toc)?;
        Ok(Self::new(html))
    }

    fn new(html: Element) -> Self {
        Self { html }
    }

    /// Return root element of markdown.
    pub fn root_element(&self) -> &Element {
        &self.html
    }

    /// Find included images.
    pub fn images(&self) -> Vec<PathBuf> {
        let mut names = vec![];
        for e in Self::visit(&self.html) {
            if e.tag() == ElementTag::Img {
                if let Some(attr) = e.attr("src") {
                    if let Some(href) = attr.value() {
                        names.push(PathBuf::from(&href));
                    }
                }
            }
        }
        names
    }

    /// Turn an element tree into a flat vector.
    pub fn visit(e: &Element) -> Vec<&Element> {
        let mut elements = vec![];
        Self::visit_helper(e, &mut elements);
        elements
    }

    fn visit_helper<'a>(e: &'a Element, elements: &mut Vec<&'a Element>) {
        elements.push(e);
        for child in e.children() {
            if let Content::Elt(ee) = child {
                Self::visit_helper(ee, elements);
            }
        }
    }

    /// Find classes used for fenced blocks.
    pub fn block_classes(&self) -> HashSet<String> {
        let mut classes: HashSet<String> = HashSet::new();

        for e in Self::visit(&self.html) {
            if e.tag() == ElementTag::Pre {
                if let Some(attr) = e.attr("class") {
                    if let Some(value) = attr.value() {
                        classes.insert(value.into());
                    }
                }
            }
        }

        classes
    }

    /// Typeset.
    pub fn typeset(
        &mut self,
        _style: Style,
        template: Option<&str>,
        bindings: &Bindings,
    ) -> Warnings {
        let result = typeset_element(&self.html, template, bindings);
        if let Ok(html) = result {
            self.html = html;
            Warnings::default()
        } else {
            // FIXME: handle warnings in some way
            Warnings::default()
        }
    }

    /// Find scenarios.
    pub fn scenarios(&self) -> Result<Vec<Scenario>, SubplotError> {
        let mut elements = vec![];
        for e in Self::visit(&self.html) {
            if let Some(se) = Self::is_structure_element(e) {
                elements.push(se);
            }
        }

        let mut scenarios: Vec<Scenario> = vec![];

        let mut i = 0;
        while i < elements.len() {
            let (maybe, new_i) = extract_scenario(&elements[i..])?;
            if let Some(scen) = maybe {
                scenarios.push(scen);
            }
            i += new_i;
        }
        trace!("Metadata::scenarios: found {} scenarios", scenarios.len());
        Ok(scenarios)
    }

    fn is_structure_element(e: &Element) -> Option<StructureElement> {
        match e.tag() {
            ElementTag::H1 => Some(StructureElement::heading(e, 1)),
            ElementTag::H2 => Some(StructureElement::heading(e, 2)),
            ElementTag::H3 => Some(StructureElement::heading(e, 3)),
            ElementTag::H4 => Some(StructureElement::heading(e, 4)),
            ElementTag::H5 => Some(StructureElement::heading(e, 5)),
            ElementTag::H6 => Some(StructureElement::heading(e, 6)),
            ElementTag::Pre => {
                if e.has_attr("class", "scenario") {
                    Some(StructureElement::snippet(e))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Find embedded files.
    pub fn embedded_files(&self) -> Result<EmbeddedFiles, MdError> {
        let mut files = EmbeddedFiles::default();

        for e in Self::visit(&self.html) {
            if let MaybeEmbeddedFile::IsFile(file) = embedded_file(e)? {
                files.push(file);
            }
        }

        Ok(files)
    }

    /// Find all code blocks which have identifiers and return them
    pub fn named_blocks(&self) -> impl Iterator<Item = &Element> {
        Self::visit(&self.html)
            .into_iter()
            .filter(|e| e.tag() == ElementTag::Pre && e.attr("id").is_some())
    }
}

// A structure element in the document: a heading or a scenario snippet.
#[derive(Debug)]
enum StructureElement {
    // Headings consist of the text and the level of the heading.
    Heading(String, i64, Location),

    // Scenario snippets consist just of the unparsed text.
    Snippet(String, Location),
}

impl StructureElement {
    fn heading(e: &Element, level: i64) -> Self {
        if let Some(title) = e.find_descendant("class", "heading-text") {
            Self::Heading(title.content(), level, title.location())
        } else {
            Self::Heading(e.content(), level, e.location())
        }
    }

    fn snippet(e: &Element) -> Self {
        Self::Snippet(e.content(), e.location())
    }
}

enum MaybeEmbeddedFile {
    IsFile(EmbeddedFile),
    NotFile,
}

fn embedded_file(e: &Element) -> Result<MaybeEmbeddedFile, MdError> {
    if e.tag() != ElementTag::Pre {
        return Ok(MaybeEmbeddedFile::NotFile);
    }

    if !e.has_attr("class", "file") {
        return Ok(MaybeEmbeddedFile::NotFile);
    }

    let id = e.attr("id");
    if id.is_none() {
        return Ok(MaybeEmbeddedFile::NotFile);
    }
    let id = id.unwrap();
    if id.value().is_none() {
        return Err(MdError::NoIdValue(e.location()));
    }
    let id = id.value().unwrap();
    if id.is_empty() {
        return Err(MdError::NoIdValue(e.location()));
    }

    // The contents we get from the pulldown_cmark parser for a code
    // block will always end in a newline, unless the block is empty.
    // This is different from the parser we previously used, which
    // didn't end in a newline, if the contents is exactly one line.
    // The add-newline attribute was designed for the previous parser
    // behavior, and so its interpretations for new new parser is a
    // little less straightforward. To avoid convoluted logic, we
    // remove the newline if it's there before obeying add-newline.
    let mut contents = e.content();
    if contents.ends_with('\n') {
        contents.truncate(contents.len() - 1);
    }
    let addnl = AddNewline::parse(e.attr("add-newline"), e.location());
    match addnl? {
        AddNewline::No => {
            // Newline already isn't there.
        }
        AddNewline::Yes => {
            // Add newline.
            contents.push('\n');
        }
        AddNewline::Auto => {
            // Add newline if not there.
            if !contents.ends_with('\n') {
                contents.push('\n');
            }
        }
    };

    Ok(MaybeEmbeddedFile::IsFile(EmbeddedFile::new(
        id.into(),
        contents,
    )))
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
enum AddNewline {
    Auto,
    Yes,
    No,
}

impl AddNewline {
    fn parse(attr: Option<&Attribute>, loc: Location) -> Result<Self, MdError> {
        if let Some(attr) = attr {
            if let Some(value) = attr.value() {
                let value = match value {
                    "yes" => Self::Yes,
                    "no" => Self::No,
                    "auto" => Self::Auto,
                    _ => return Err(MdError::BadAddNewline(value.into(), loc)),
                };
                return Ok(value);
            }
        };
        Ok(Self::Auto)
    }
}

fn extract_scenario(e: &[StructureElement]) -> Result<(Option<Scenario>, usize), SubplotError> {
    if e.is_empty() {
        // If we get here, it's a programming error.
        panic!("didn't expect empty list of elements");
    }

    match &e[0] {
        StructureElement::Snippet(_, loc) => Err(SubplotError::ScenarioBeforeHeading(loc.clone())),
        StructureElement::Heading(title, level, loc) => {
            let mut scen = Scenario::new(title, loc.clone());
            for (i, item) in e.iter().enumerate().skip(1) {
                match item {
                    StructureElement::Heading(_, level2, _loc) => {
                        let is_subsection = *level2 > *level;
                        if is_subsection {
                            if scen.has_steps() {
                            } else {
                                return Ok((None, i));
                            }
                        } else if scen.has_steps() {
                            return Ok((Some(scen), i));
                        } else {
                            return Ok((None, i));
                        }
                    }
                    StructureElement::Snippet(text, loc) => {
                        let steps = parse_scenario_snippet(text, loc)?;
                        for step in steps {
                            scen.add(&step);
                        }
                    }
                }
            }
            if scen.has_steps() {
                Ok((Some(scen), e.len()))
            } else {
                Ok((None, e.len()))
            }
        }
    }
}

/// Errors returned from the module.
#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum MdError {
    /// Tried to treat a non-PRE element as an embedded file.
    #[error("{1}: tried to treat wrong kind of element as an embedded file: {0}")]
    NotCodeBlockElement(String, Location),

    /// Code block lacks the "file" attribute.
    #[error("{0}; code block is not a file")]
    NotFile(Location),

    /// Code block lacks an identifier to use as the filename.
    #[error("{0}: code block lacks a filename identifier")]
    NoId(Location),

    /// Identifier is empty.
    #[error("{0}: code block has an empty filename identifier")]
    NoIdValue(Location),

    /// Value of add-newline attribute is not understood.
    #[error("{1}: value of add-newline attribute is not understood: {0}")]
    BadAddNewline(String, Location),
}

#[cfg(test)]
mod test_extract {
    use super::extract_scenario;
    use super::Location;
    use super::StructureElement;
    use crate::Scenario;
    use crate::SubplotError;

    fn h(title: &str, level: i64) -> StructureElement {
        StructureElement::Heading(title.to_string(), level, Location::unknown())
    }

    fn s(text: &str) -> StructureElement {
        StructureElement::Snippet(text.to_string(), Location::unknown())
    }

    fn check_result(
        r: Result<(Option<Scenario>, usize), SubplotError>,
        title: Option<&str>,
        i: usize,
    ) {
        assert!(r.is_ok());
        let (actual_scen, actual_i) = r.unwrap();
        if title.is_none() {
            assert!(actual_scen.is_none());
        } else {
            assert!(actual_scen.is_some());
            let scen = actual_scen.unwrap();
            assert_eq!(scen.title(), title.unwrap());
        }
        assert_eq!(actual_i, i);
    }

    #[test]
    fn returns_nothing_if_there_is_no_scenario() {
        let elements: Vec<StructureElement> = vec![h("title", 1)];
        let r = extract_scenario(&elements);
        check_result(r, None, 1);
    }

    #[test]
    fn returns_scenario_if_there_is_one() {
        let elements = vec![h("title", 1), s("given something")];
        let r = extract_scenario(&elements);
        check_result(r, Some("title"), 2);
    }

    #[test]
    fn skips_scenarioless_section_in_favour_of_same_level() {
        let elements = vec![h("first", 1), h("second", 1), s("given something")];
        let r = extract_scenario(&elements);
        check_result(r, None, 1);
        let r = extract_scenario(&elements[1..]);
        check_result(r, Some("second"), 2);
    }

    #[test]
    fn returns_parent_section_with_scenario_snippet() {
        let elements = vec![
            h("1", 1),
            s("given something"),
            h("1.1", 2),
            s("when something"),
            h("2", 1),
        ];
        let r = extract_scenario(&elements);
        check_result(r, Some("1"), 4);
        let r = extract_scenario(&elements[4..]);
        check_result(r, None, 1);
    }

    #[test]
    fn skips_scenarioless_parent_heading() {
        let elements = vec![h("1", 1), h("1.1", 2), s("given something"), h("2", 1)];

        let r = extract_scenario(&elements);
        check_result(r, None, 1);

        let r = extract_scenario(&elements[1..]);
        check_result(r, Some("1.1"), 2);

        let r = extract_scenario(&elements[3..]);
        check_result(r, None, 1);
    }

    #[test]
    fn skips_scenarioless_deeper_headings() {
        let elements = vec![h("1", 1), h("1.1", 2), h("2", 1), s("given something")];

        let r = extract_scenario(&elements);
        check_result(r, None, 1);

        let r = extract_scenario(&elements[1..]);
        check_result(r, None, 1);

        let r = extract_scenario(&elements[2..]);
        check_result(r, Some("2"), 2);
    }

    #[test]
    fn returns_error_if_scenario_has_no_title() {
        let elements = vec![s("given something")];
        let r = extract_scenario(&elements);
        match r {
            Err(SubplotError::ScenarioBeforeHeading(_)) => (),
            _ => panic!("unexpected result {:?}", r),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::toc::TableOfContents;

    use super::{AddNewline, Attribute, Location, Markdown, MdError};
    use std::path::{Path, PathBuf};

    #[test]
    fn loads_empty_doc() {
        let mut toc = TableOfContents::default();
        let md = Markdown::new_from_str(Path::new(""), "", &mut toc).unwrap();
        assert!(md.html.content().is_empty());
    }

    #[test]
    fn finds_no_images_in_empty_doc() {
        let mut toc = TableOfContents::default();
        let md = Markdown::new_from_str(Path::new(""), "", &mut toc).unwrap();
        assert!(md.images().is_empty());
    }

    #[test]
    fn finds_images() {
        let mut toc = TableOfContents::default();
        let md = Markdown::new_from_str(
            Path::new(""),
            r#"
![alt text](filename.jpg)
"#,
            &mut toc,
        )
        .unwrap();
        assert_eq!(md.images(), vec![PathBuf::from("filename.jpg")]);
    }

    #[test]
    fn finds_no_blocks_in_empty_doc() {
        let mut toc = TableOfContents::default();
        let md = Markdown::new_from_str(Path::new(""), "", &mut toc).unwrap();
        assert!(md.block_classes().is_empty());
    }

    #[test]
    fn finds_no_classes_when_no_blocks_have_them() {
        let mut toc = TableOfContents::default();
        let md = Markdown::new_from_str(
            Path::new(""),
            r#"
~~~
~~~
"#,
            &mut toc,
        )
        .unwrap();
        assert!(md.block_classes().is_empty());
    }

    #[test]
    fn finds_block_classes() {
        let mut toc = TableOfContents::default();
        let md = Markdown::new_from_str(
            Path::new(""),
            r#"
~~~scenario
~~~
"#,
            &mut toc,
        )
        .unwrap();
        let classes: Vec<String> = md.block_classes().iter().map(|s| s.into()).collect();
        assert_eq!(classes, vec!["scenario"]);
    }

    #[test]
    fn finds_no_scenarios_in_empty_doc() {
        let mut toc = TableOfContents::default();
        let md = Markdown::new_from_str(Path::new(""), "", &mut toc).unwrap();
        let scenarios = md.scenarios().unwrap();
        assert!(scenarios.is_empty());
    }

    #[test]
    fn finds_scenarios() {
        let mut toc = TableOfContents::default();
        let md = Markdown::new_from_str(
            Path::new(""),
            r#"
# Super trooper

~~~scenario
given ABBA
~~~
"#,
            &mut toc,
        )
        .unwrap();
        let scenarios = md.scenarios().unwrap();
        assert_eq!(scenarios.len(), 1);
        let scen = scenarios.first().unwrap();
        assert_eq!(scen.title(), "Super trooper");
        let steps = scen.steps();
        assert_eq!(steps.len(), 1);
        let step = steps.first().unwrap();
        assert_eq!(step.kind(), crate::StepKind::Given);
        assert_eq!(step.text(), "ABBA");
    }

    #[test]
    fn finds_no_embedded_files_in_empty_doc() {
        let mut toc = TableOfContents::default();
        let md = Markdown::new_from_str(Path::new(""), "", &mut toc).unwrap();
        let files = md.embedded_files();
        assert!(files.unwrap().files().is_empty());
    }

    #[test]
    fn finds_embedded_files() {
        let mut toc = TableOfContents::default();
        let md = Markdown::new_from_str(
            Path::new(""),
            r#"
~~~{#fileid .file .text}
hello, world
~~~
"#,
            &mut toc,
        )
        .unwrap();
        let files = md.embedded_files().unwrap();
        assert_eq!(files.files().len(), 1);
        let file = files.files().first().unwrap();
        assert_eq!(file.filename(), "fileid");
        assert_eq!(file.contents(), "hello, world\n");
    }

    #[test]
    fn parses_no_auto_newline_as_auto() {
        assert_eq!(
            AddNewline::parse(None, Location::unknown()).unwrap(),
            AddNewline::Auto
        );
    }

    #[test]
    fn parses_auto_as_auto() {
        let attr = Attribute::new("add-newline", "auto");
        assert_eq!(
            AddNewline::parse(Some(&attr), Location::unknown()).unwrap(),
            AddNewline::Auto
        );
    }

    #[test]
    fn parses_yes_as_yes() {
        let attr = Attribute::new("add-newline", "yes");
        assert_eq!(
            AddNewline::parse(Some(&attr), Location::unknown()).unwrap(),
            AddNewline::Yes
        );
    }

    #[test]
    fn parses_no_as_no() {
        let attr = Attribute::new("add-newline", "no");
        assert_eq!(
            AddNewline::parse(Some(&attr), Location::unknown()).unwrap(),
            AddNewline::No
        );
    }

    #[test]
    fn parses_empty_as_error() {
        let attr = Attribute::new("add-newline", "");
        assert_eq!(
            AddNewline::parse(Some(&attr), Location::unknown()),
            Err(MdError::BadAddNewline("".into(), Location::unknown()))
        );
    }

    #[test]
    fn parses_garbage_as_error() {
        let attr = Attribute::new("add-newline", "garbage");
        assert_eq!(
            AddNewline::parse(Some(&attr), Location::unknown()),
            Err(MdError::BadAddNewline(
                "garbage".into(),
                Location::unknown()
            ))
        );
    }
}
