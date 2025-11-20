//! A representation of HTML using Rust types.

#![deny(missing_docs)]

use html_escape::{encode_double_quoted_attribute, encode_text};
use log::{debug, trace};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Write as _;
use std::io::Write;
use std::path::{Path, PathBuf};

const DOCTYPE: &str = "<!DOCTYPE html>";

/// A HTML page, consisting of a head and a body.
#[derive(Debug)]
pub struct HtmlPage {
    head: Element,
    body: Element,
}

impl Default for HtmlPage {
    fn default() -> Self {
        Self {
            head: Element::new(ElementTag::Head),
            body: Element::new(ElementTag::Body),
        }
    }
}

impl HtmlPage {
    /// Create a new HTML page from a head and a body element.
    pub fn new(head: Element, body: Element) -> Self {
        Self { head, body }
    }

    /// Return the page's head element.
    pub fn head(&self) -> &Element {
        &self.head
    }

    /// Return the page's body element.
    pub fn body(&self) -> &Element {
        &self.body
    }

    /// Try to serialize an HTML page into HTML text.
    pub fn serialize(&self) -> Result<String, HtmlError> {
        let mut html = Element::new(ElementTag::Html);
        html.push_child(Content::Elt(self.head.clone()));
        let mut body = Element::new(ElementTag::Body);
        body.push_child(Content::Elt(self.body.clone()));
        html.push_child(Content::Elt(body));
        let html = html.serialize()?;
        Ok(format!("{}\n{}", DOCTYPE, html))
    }

    /// Try to write an HTML page as text into a file.
    pub fn write(&self, filename: &Path) -> Result<(), HtmlError> {
        if let Some(parent) = filename.parent() {
            trace!("parent: {}", parent.display());
            if !parent.exists() {
                debug!("creating directory {}", parent.display());
                std::fs::create_dir_all(parent)
                    .map_err(|e| HtmlError::CreateDir(parent.into(), e))?;
            }
        }

        trace!("writing HTML: {}", filename.display());
        let mut f = std::fs::File::create(filename)
            .map_err(|e| HtmlError::CreateFile(filename.into(), e))?;
        let html = self.serialize()?;
        f.write_all(html.as_bytes())
            .map_err(|e| HtmlError::FileWrite(filename.into(), e))?;
        Ok(())
    }
}

/// Return text of a sequence of contents as a string.
pub fn as_plain_text(content: &[Content]) -> String {
    fn as_helper(buf: &mut String, c: &Content) {
        match c {
            Content::Text(s) => buf.push_str(s),
            Content::Html(s) => buf.push_str(s),
            Content::Elt(e) => {
                for child in e.children() {
                    as_helper(buf, child);
                }
            }
        }
    }

    let mut buf = String::new();
    for c in content {
        as_helper(&mut buf, c);
    }
    buf
}

/// An HTML element.
#[derive(Debug, Clone)]
pub struct Element {
    loc: Option<Location>,
    tag: ElementTag,
    attrs: Vec<Attribute>,
    children: Vec<Content>,
}

impl Element {
    /// Create a new element.
    pub fn new(tag: ElementTag) -> Self {
        Self {
            loc: None,
            tag,
            attrs: vec![],
            children: vec![],
        }
    }

    /// Add location to an element.
    pub fn with_location(mut self, loc: Location) -> Self {
        self.loc = Some(loc);
        self
    }

    /// Set location.
    pub fn set_location(&mut self, loc: Location) {
        self.loc = Some(loc);
    }

    /// Get location.
    pub fn location(&self) -> Location {
        if let Some(loc) = &self.loc {
            loc.clone()
        } else {
            Location::unknown()
        }
    }

    /// Set the block attributes for an element.
    pub fn set_block_attributes(&mut self, block_attrs: Vec<BlockAttr>) {
        for block_attr in block_attrs {
            let attr = Attribute::from(block_attr);
            self.attrs.push(attr);
        }
    }

    /// Add a new attribute. If an attribute with the same name
    /// already exists, append to its value.
    pub fn push_attribute(&mut self, attr: Attribute) {
        self.attrs.push(attr);
    }

    /// Add a new attribute. If an attribute of the same name already
    /// exists on the element, remove it first.
    pub fn push_unique_attribute(&mut self, attr: Attribute) {
        for (i, a) in self.attrs.iter().enumerate() {
            if a.name == attr.name {
                self.attrs.remove(i);
                break;
            }
        }

        self.attrs.push(attr);
    }

    /// Drop all attributes with a given name.
    pub fn drop_attributes(&mut self, unwanted: &[&str]) {
        for uw in unwanted {
            self.attrs.retain(|a| a.name() != *uw);
        }
    }

    /// Append a new child to the element.
    pub fn push_child(&mut self, child: Content) {
        self.children.push(child);
    }

    /// Return an element's tag.
    pub fn tag(&self) -> ElementTag {
        self.tag
    }

    /// All attributes.
    pub fn all_attrs(&self) -> &[Attribute] {
        &self.attrs
    }

    /// Return value of a named attribute, if any.
    pub fn attr(&self, name: &str) -> Option<&Attribute> {
        self.attrs.iter().find(|a| a.name() == name)
    }

    /// Has an attribute with a specific value?
    pub fn has_attr(&self, name: &str, wanted: &str) -> bool {
        self.attrs
            .iter()
            .filter(|a| a.name() == name && a.value() == Some(wanted))
            .count()
            > 0
    }

    /// Return the concatenated text content of direct children,
    /// ignoring any elements.
    pub fn content(&self) -> String {
        let mut buf = String::new();
        for child in self.children() {
            buf.push_str(&child.content());
        }
        buf
    }

    /// Return all the children of an element.
    pub fn children(&self) -> &[Content] {
        &self.children
    }

    /// Find first descendant element with that has an attribute with
    /// a specific value. Not necessarily direct child. Search is
    /// element first, then recursively the element's children that
    /// are elements themselves.
    pub fn find_descendant(&self, name: &str, value: &str) -> Option<&Self> {
        if self.has_attr(name, value) {
            return Some(self);
        }

        for child in self.children() {
            if let Content::Elt(e) = child {
                if let Some(it) = e.find_descendant(name, value) {
                    return Some(it);
                }
            }
        }

        None
    }

    /// Try to add an alt attribute to an img element.
    pub fn fix_up_img_alt(&mut self) {
        if self.tag == ElementTag::Img {
            if !self.attrs.iter().any(|a| a.name() == "alt") {
                let alt = as_plain_text(self.children());
                self.push_attribute(Attribute::new("alt", &alt));
                self.children.clear();
            }
        } else {
            for child in self.children.iter_mut() {
                if let Content::Elt(kid) = child {
                    kid.fix_up_img_alt();
                }
            }
        }
    }

    /// Serialize an element into HTML text.
    pub fn serialize(&self) -> Result<String, HtmlError> {
        let mut buf = String::new();
        self.serialize_to_buf_without_added_newlines(&mut buf)
            .map_err(HtmlError::Format)?;
        Ok(buf)
    }

    fn serialize_to_buf_without_added_newlines(
        &self,
        buf: &mut String,
    ) -> Result<(), std::fmt::Error> {
        if self.tag.can_self_close() && self.children.is_empty() {
            write!(buf, "<{}", self.tag.name())?;
            self.serialize_attrs_to_buf(buf)?;
            write!(buf, "/>")?;
        } else {
            write!(buf, "<{}", self.tag.name())?;
            self.serialize_attrs_to_buf(buf)?;
            write!(buf, ">")?;
            for c in self.children() {
                match c {
                    Content::Text(s) => buf.push_str(&encode_text(s)),
                    Content::Elt(e) => e.serialize_to_buf_adding_block_newline(buf)?,
                    Content::Html(s) => buf.push_str(s),
                }
            }
            write!(buf, "</{}>", self.tag.name())?;
        }
        Ok(())
    }

    fn serialize_to_buf_adding_block_newline(
        &self,
        buf: &mut String,
    ) -> Result<(), std::fmt::Error> {
        if self.tag.is_block() {
            writeln!(buf)?;
        }
        self.serialize_to_buf_without_added_newlines(buf)
    }

    fn serialize_attrs_to_buf(&self, buf: &mut String) -> Result<(), std::fmt::Error> {
        let mut attrs = Attributes::default();
        for attr in self.attrs.iter() {
            attrs.push(attr);
        }

        for (name, value) in attrs.iter() {
            write!(buf, " {}", name)?;
            if !value.is_empty() {
                write!(buf, "=\"{}\"", encode_double_quoted_attribute(value))?;
            }
        }
        Ok(())
    }
}

/// The tag of an HTML element.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[allow(missing_docs)]
pub enum ElementTag {
    Html,
    Head,
    Meta,
    Body,
    Div,
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,
    P,
    Ol,
    Ul,
    Li,
    Link,
    Blockquote,
    Pre,
    Em,
    Strong,
    Del,
    A,
    Img,
    Table,
    Title,
    Th,
    Tr,
    Td,
    Br,
    Hr,
    Code,
    Span,
    Style,
}

impl ElementTag {
    /// Name of the tag.
    pub fn name(&self) -> &str {
        match self {
            Self::Html => "html",
            Self::Head => "head",
            Self::Meta => "meta",
            Self::Body => "body",
            Self::Div => "div",
            Self::H1 => "h1",
            Self::H2 => "h2",
            Self::H3 => "h3",
            Self::H4 => "h4",
            Self::H5 => "h5",
            Self::H6 => "h6",
            Self::P => "p",
            Self::Ol => "ol",
            Self::Ul => "ul",
            Self::Li => "li",
            Self::Link => "link",
            Self::Blockquote => "blockquote",
            Self::Pre => "pre",
            Self::Em => "em",
            Self::Strong => "strong",
            Self::Del => "del",
            Self::A => "a",
            Self::Img => "img",
            Self::Table => "table",
            Self::Th => "th",
            Self::Title => "title",
            Self::Tr => "tr",
            Self::Td => "td",
            Self::Br => "br",
            Self::Hr => "hr",
            Self::Code => "code",
            Self::Span => "span",
            Self::Style => "style",
        }
    }

    fn is_block(&self) -> bool {
        matches!(
            self,
            Self::Html
                | Self::Head
                | Self::Meta
                | Self::Body
                | Self::Div
                | Self::H1
                | Self::H2
                | Self::H3
                | Self::H4
                | Self::H5
                | Self::H6
                | Self::P
                | Self::Ol
                | Self::Ul
                | Self::Li
                | Self::Blockquote
                | Self::Table
                | Self::Th
                | Self::Tr
                | Self::Br
                | Self::Hr
        )
    }

    fn can_self_close(&self) -> bool {
        matches!(
            self,
            Self::Br | Self::Hr | Self::Img | Self::Link | Self::Meta
        )
    }
}

#[cfg(test)]
mod test_tag {
    use super::ElementTag;

    #[test]
    fn can_self_close() {
        assert!(ElementTag::Br.can_self_close());
        assert!(ElementTag::Hr.can_self_close());
        assert!(ElementTag::Img.can_self_close());
        assert!(ElementTag::Link.can_self_close());
        assert!(ElementTag::Meta.can_self_close());
    }

    #[test]
    fn cannot_self_close() {
        assert!(!ElementTag::Html.can_self_close());
        assert!(!ElementTag::Head.can_self_close());
        assert!(!ElementTag::Body.can_self_close());
        assert!(!ElementTag::Div.can_self_close());
        assert!(!ElementTag::H1.can_self_close());
        assert!(!ElementTag::H2.can_self_close());
        assert!(!ElementTag::H3.can_self_close());
        assert!(!ElementTag::H4.can_self_close());
        assert!(!ElementTag::H5.can_self_close());
        assert!(!ElementTag::H6.can_self_close());
        assert!(!ElementTag::P.can_self_close());
        assert!(!ElementTag::Ol.can_self_close());
        assert!(!ElementTag::Ul.can_self_close());
        assert!(!ElementTag::Li.can_self_close());
        assert!(!ElementTag::Blockquote.can_self_close());
        assert!(!ElementTag::Pre.can_self_close());
        assert!(!ElementTag::Em.can_self_close());
        assert!(!ElementTag::Strong.can_self_close());
        assert!(!ElementTag::Del.can_self_close());
        assert!(!ElementTag::A.can_self_close());
        assert!(!ElementTag::Table.can_self_close());
        assert!(!ElementTag::Title.can_self_close());
        assert!(!ElementTag::Th.can_self_close());
        assert!(!ElementTag::Tr.can_self_close());
        assert!(!ElementTag::Td.can_self_close());
        assert!(!ElementTag::Code.can_self_close());
        assert!(!ElementTag::Span.can_self_close());
        assert!(!ElementTag::Style.can_self_close());
    }
}

#[derive(Debug, Default, Clone)]
struct Attributes {
    attrs: HashMap<String, String>,
}

impl Attributes {
    fn push(&mut self, attr: &Attribute) {
        if let Some(new_value) = attr.value() {
            if let Some(old_value) = self.attrs.get_mut(attr.name()) {
                assert!(!old_value.is_empty());
                old_value.push(' ');
                old_value.push_str(new_value);
            } else {
                self.attrs.insert(attr.name().into(), new_value.into());
            }
        } else {
            assert!(!self.attrs.contains_key(attr.name()));
            self.attrs.insert(attr.name().into(), "".into());
        }
    }

    fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
        self.attrs.iter()
    }
}

/// An attribute of an HTML element.
#[derive(Clone, Debug)]
pub struct Attribute {
    name: String,
    value: Option<String>,
}

impl Attribute {
    /// Create a new element attribute.
    pub fn new(name: &str, value: &str) -> Self {
        Self {
            name: name.into(),
            value: Some(value.into()),
        }
    }

    /// Return the name of the attribute.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the value of the attribute, if any.
    pub fn value(&self) -> Option<&str> {
        self.value.as_deref()
    }
}

impl From<BlockAttr> for Attribute {
    fn from(block_attr: BlockAttr) -> Self {
        match block_attr {
            BlockAttr::Id(v) => Self::new("id", &v),
            BlockAttr::Class(v) => Self::new("class", &v),
            BlockAttr::KeyValue(k, v) => Self::new(&k, &v),
        }
    }
}

/// Content in HTML.
#[derive(Clone, Debug)]
pub enum Content {
    /// Arbitrary text.
    Text(String),

    /// An HTML element.
    Elt(Element),

    /// Arbitrary HTML text.
    Html(String),
}

impl Content {
    fn content(&self) -> String {
        match self {
            Self::Text(s) => s.clone(),
            Self::Elt(e) => e.content(),
            Self::Html(h) => h.clone(),
        }
    }
}

/// Location of element in source file.
#[derive(Debug, Clone, Eq, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Location {
    /// A known location.
    Known {
        /// Name of file.
        filename: PathBuf,
        /// Line in file.
        line: usize,
        /// Column in line.
        col: usize,
    },
    /// An unknown location.
    Unknown,
}

impl Location {
    /// Create a new location.
    pub fn new(filename: &Path, line: usize, col: usize) -> Self {
        Self::Known {
            filename: filename.into(),
            line,
            col,
        }
    }

    /// Create an unknown location.
    pub fn unknown() -> Self {
        Self::Unknown
    }

    /// Report name of source file from where this element comes from.
    pub fn filename(&self) -> &Path {
        if let Self::Known {
            filename,
            line: _,
            col: _,
        } = self
        {
            filename
        } else {
            Path::new("")
        }
    }

    /// Report row and column in source where this element comes from.
    pub fn rowcol(&self) -> (usize, usize) {
        if let Self::Known {
            filename: _,
            line,
            col,
        } = self
        {
            (*line, *col)
        } else {
            (0, 0)
        }
    }
}

impl std::fmt::Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        if let Self::Known {
            filename,
            line,
            col,
        } = self
        {
            write!(f, "{}:{}:{}", filename.display(), line, col)
        } else {
            write!(f, "(unknown location)")
        }
    }
}

/// Errors from the `html` module.
#[derive(Debug, thiserror::Error)]
pub enum HtmlError {
    /// Failed to create a directory.
    #[error("failed to create directory {0}")]
    CreateDir(PathBuf, #[source] std::io::Error),

    /// Failed to create a file.
    #[error("failed to create file {0}")]
    CreateFile(PathBuf, #[source] std::io::Error),

    /// Failed to write to a file.
    #[error("failed to write to file {0}")]
    FileWrite(PathBuf, #[source] std::io::Error),

    /// Input contains an attempt to use a definition list in
    /// Markdown.
    #[error("{0}: attempt to use definition lists in Markdown")]
    DefinitionList(Location),

    /// String formatting error. This is likely a programming error.
    #[error("string formatting error: {0}")]
    Format(#[source] std::fmt::Error),

    /// Math is not supported in Subplot.
    #[error("math markup used in markdown")]
    Math,

    /// Metadata blocks are not supported in Subplot.
    #[error("metadata block use in markdown")]
    Metadata,

    /// Error generating a table of content entry.
    #[error(transparent)]
    ToC(#[from] crate::toc::ToCError),
}

/// Code block attribute.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum BlockAttr {
    /// An identifier.
    Id(String),
    /// A class.
    Class(String),
    /// A key/value pair.
    KeyValue(String, String),
}

impl BlockAttr {
    fn id(s: &str) -> Self {
        Self::Id(s.into())
    }

    fn class(s: &str) -> Self {
        Self::Class(s.into())
    }

    fn key_value(k: &str, v: &str) -> Self {
        Self::KeyValue(k.into(), v.into())
    }

    /// Parse a fenced code block tag.
    pub fn parse(attrs: &str) -> Vec<Self> {
        let mut result = vec![];
        for word in Self::parse_words(attrs) {
            let attr = Self::parse_word(word);
            result.push(attr);
        }
        result
    }

    fn parse_words(attrs: &str) -> impl Iterator<Item = &str> {
        if attrs.starts_with('{') && attrs.ends_with('}') {
            attrs[1..attrs.len() - 1].split_ascii_whitespace()
        } else {
            attrs.split_ascii_whitespace()
        }
    }

    fn parse_word(word: &str) -> Self {
        if let Some(id) = word.strip_prefix('#') {
            Self::id(id)
        } else if let Some(class) = word.strip_prefix('.') {
            Self::class(class)
        } else if let Some((key, value)) = word.split_once('=') {
            Self::key_value(key, value)
        } else {
            Self::class(word)
        }
    }
}

#[cfg(test)]
mod test_block_attr {
    use super::BlockAttr;

    #[test]
    fn empty_string() {
        assert_eq!(BlockAttr::parse(""), vec![]);
    }

    #[test]
    fn plain_word() {
        assert_eq!(
            BlockAttr::parse("foo"),
            vec![BlockAttr::Class("foo".into())]
        );
    }

    #[test]
    fn dot_word() {
        assert_eq!(
            BlockAttr::parse(".foo"),
            vec![BlockAttr::Class("foo".into())]
        );
    }

    #[test]
    fn hash_word() {
        assert_eq!(BlockAttr::parse("#foo"), vec![BlockAttr::Id("foo".into())]);
    }

    #[test]
    fn key_value() {
        assert_eq!(
            BlockAttr::parse("foo=bar"),
            vec![BlockAttr::KeyValue("foo".into(), "bar".into())]
        );
    }

    #[test]
    fn several() {
        assert_eq!(
            BlockAttr::parse("{#foo .bar foobar yo=yoyo}"),
            vec![
                BlockAttr::Id("foo".into()),
                BlockAttr::Class("bar".into()),
                BlockAttr::Class("foobar".into()),
                BlockAttr::KeyValue("yo".into(), "yoyo".into()),
            ]
        );
    }
}
