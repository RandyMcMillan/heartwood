//! Parse markdown into an HTML representation.

use std::path::Path;

use line_col::LineColLookup;
use log::trace;
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::{
    html::{
        as_plain_text, Attribute, BlockAttr, Content, Element, ElementTag, HtmlError, Location,
    },
    toc::TableOfContents,
};

/// Parse Markdown text into an HTML element.
pub fn parse(
    filename: &Path,
    markdown: &str,
    toc: &mut TableOfContents,
) -> Result<Element, HtmlError> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_HEADING_ATTRIBUTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_TASKLISTS);
    let p = Parser::new_ext(markdown, options).into_offset_iter();
    let linecol = LineColLookup::new(markdown);
    let mut stack = Stack::new();
    stack.push(Element::new(ElementTag::Div));
    let mut table_cell_tag = vec![];
    for (event, loc) in p {
        trace!("event {:?}", event);
        let (line, col) = linecol.get(loc.start);
        let loc = Location::new(filename, line, col);
        match event {
            Event::DisplayMath(_) | Event::InlineMath(_) => return Err(HtmlError::Math),
            Event::Start(tag) => match tag {
                Tag::HtmlBlock => (),
                Tag::MetadataBlock(_) => return Err(HtmlError::Metadata),
                Tag::DefinitionList | Tag::DefinitionListTitle | Tag::DefinitionListDefinition => {
                    return Err(HtmlError::DefinitionList(loc));
                }
                Tag::Paragraph => stack.push_tag(ElementTag::P, loc),
                Tag::Heading {
                    level,
                    id,
                    classes,
                    attrs,
                } => {
                    let tag = match level {
                        HeadingLevel::H1 => ElementTag::H1,
                        HeadingLevel::H2 => ElementTag::H2,
                        HeadingLevel::H3 => ElementTag::H3,
                        HeadingLevel::H4 => ElementTag::H4,
                        HeadingLevel::H5 => ElementTag::H5,
                        HeadingLevel::H6 => ElementTag::H6,
                    };
                    let mut h = Element::new(tag).with_location(loc);
                    if let Some(id) = id {
                        let id = id.to_string();
                        h.push_unique_attribute(Attribute::new("id", &id));
                    }
                    if !classes.is_empty() {
                        let mut names = String::new();
                        for c in classes {
                            if !names.is_empty() {
                                names.push(' ');
                            }
                            names.push_str(c.to_string().as_str());
                        }
                        h.push_attribute(Attribute::new("class", &names));
                    }
                    for (name, value) in attrs.iter() {
                        let name = name.to_string();
                        let value = value.clone().map(|v| v.to_string()).unwrap_or(name.clone());
                        h.push_attribute(Attribute::new(&name, &value));
                    }
                    stack.push(h);
                }
                Tag::BlockQuote(_) => stack.push_tag(ElementTag::Blockquote, loc),
                Tag::CodeBlock(kind) => {
                    stack.push_tag(ElementTag::Pre, loc);
                    if let CodeBlockKind::Fenced(attrs) = kind {
                        let mut e = stack.pop();
                        e.set_block_attributes(BlockAttr::parse(&attrs));
                        stack.push(e);
                    }
                }
                Tag::List(None) => stack.push_tag(ElementTag::Ul, loc),
                Tag::List(Some(start)) => {
                    let mut e = Element::new(ElementTag::Ol).with_location(loc);
                    e.push_attribute(Attribute::new("start", &format!("{}", start)));
                    stack.push(e);
                }
                Tag::Item => stack.push_tag(ElementTag::Li, loc),
                Tag::FootnoteDefinition(_) => unreachable!("{:?}", tag),
                Tag::Table(_) => {
                    stack.push_tag(ElementTag::Table, loc);
                    table_cell_tag.push(ElementTag::Td);
                }
                Tag::TableRow => stack.push_tag(ElementTag::Tr, loc),
                Tag::TableHead => {
                    stack.push_tag(ElementTag::Tr, loc);
                    table_cell_tag.push(ElementTag::Th);
                }
                Tag::TableCell => {
                    let tag = table_cell_tag.pop().unwrap();
                    table_cell_tag.push(tag);
                    stack.push_tag(tag, loc);
                }
                Tag::Emphasis => stack.push_tag(ElementTag::Em, loc),
                Tag::Strong => stack.push_tag(ElementTag::Strong, loc),
                Tag::Strikethrough => stack.push_tag(ElementTag::Del, loc),
                Tag::Link {
                    link_type: _,
                    dest_url: url,
                    title,
                    id: _,
                } => {
                    let mut link = Element::new(ElementTag::A);
                    link.push_attribute(Attribute::new("href", url.as_ref()));
                    if !title.is_empty() {
                        link.push_attribute(Attribute::new("title", title.as_ref()));
                    }
                    stack.push(link);
                }
                Tag::Image {
                    link_type: _,
                    dest_url: url,
                    title,
                    id: _,
                } => {
                    let mut e = Element::new(ElementTag::Img);
                    e.push_attribute(Attribute::new("src", url.as_ref()));
                    e.push_attribute(Attribute::new("alt", title.as_ref()));
                    if !title.is_empty() {
                        e.push_attribute(Attribute::new("title", title.as_ref()));
                    }
                    stack.push(e);
                }
            },
            Event::End(tag) => match &tag {
                TagEnd::HtmlBlock => (),
                TagEnd::MetadataBlock(_) => panic!("metadata block end not handled"),
                TagEnd::DefinitionList
                | TagEnd::DefinitionListTitle
                | TagEnd::DefinitionListDefinition => {
                    return Err(HtmlError::DefinitionList(loc));
                }
                TagEnd::Paragraph => {
                    trace!("at end of paragraph, looking for definition list use");
                    let e = stack.pop();
                    let s = as_plain_text(e.children());
                    trace!("paragraph text: {:?}", s);
                    if s.starts_with(": ") || s.contains("\n: ") {
                        return Err(HtmlError::DefinitionList(loc));
                    }
                    stack.append_child(Content::Elt(e));
                }
                TagEnd::Heading(_) => {
                    // Construct a new Hx element that includes number.

                    let mut e = stack.pop();
                    let level = match e.tag() {
                        ElementTag::H1 => 1,
                        ElementTag::H2 => 2,
                        ElementTag::H3 => 3,
                        ElementTag::H4 => 4,
                        ElementTag::H5 => 5,
                        ElementTag::H6 => 6,
                        _ => {
                            unreachable!("programming error: expected a heading, got {:?}", e.tag())
                        }
                    };

                    let mut content = Element::new(ElementTag::Span);
                    content.push_unique_attribute(Attribute::new("class", "heading-text"));
                    for child in e.children() {
                        content.push_child(child.clone());
                    }

                    let id = e.attr("id").map(|a| a.value()).unwrap_or(None);

                    let h = toc.push_heading(level, &Content::Elt(content), id)?;
                    e.push_unique_attribute(Attribute::new("id", &h.slug));

                    let mut number = Element::new(ElementTag::Span);
                    number.push_attribute(Attribute::new("class", "heading-number"));
                    number.push_child(Content::Text(h.number.clone()));

                    let mut numbered = Element::new(ElementTag::Span);
                    numbered.push_child(Content::Elt(number));
                    numbered.push_child(Content::Text(" ".into()));
                    numbered.push_child(h.content.clone());

                    let mut new_e = Element::new(e.tag());
                    for attr in e.all_attrs() {
                        new_e.push_attribute(attr.clone());
                    }
                    new_e.push_child(Content::Elt(numbered));

                    stack.append_child(Content::Elt(new_e));
                }
                TagEnd::List(_)
                | TagEnd::Item
                | TagEnd::Link
                | TagEnd::Image
                | TagEnd::Emphasis
                | TagEnd::Table
                | TagEnd::TableRow
                | TagEnd::TableCell
                | TagEnd::Strong
                | TagEnd::Strikethrough
                | TagEnd::BlockQuote(_)
                | TagEnd::CodeBlock => {
                    let e = stack.pop();
                    stack.append_child(Content::Elt(e));
                }
                TagEnd::TableHead => {
                    let e = stack.pop();
                    stack.append_child(Content::Elt(e));
                    assert!(!table_cell_tag.is_empty());
                    table_cell_tag.pop();
                }
                TagEnd::FootnoteDefinition => unreachable!("{:?}", tag),
            },
            Event::Text(s) => stack.append_str(s.as_ref()),
            Event::Code(s) => {
                let mut code = Element::new(ElementTag::Code);
                code.push_child(Content::Text(s.to_string()));
                stack.append_element(code);
            }
            Event::Html(s) | Event::InlineHtml(s) => {
                stack.append_child(Content::Html(s.to_string()))
            }
            Event::FootnoteReference(s) => trace!("footnote ref {:?}", s),
            Event::SoftBreak => stack.append_str("\n"),
            Event::HardBreak => stack.append_element(Element::new(ElementTag::Br)),
            Event::Rule => stack.append_element(Element::new(ElementTag::Hr)),
            Event::TaskListMarker(done) => {
                let marker = if done {
                    "\u{2612} " // Unicode for box with X
                } else {
                    "\u{2610} " // Unicode for empty box
                };
                stack.append_str(marker);
            }
        }
    }

    let mut body = stack.pop();
    assert!(stack.is_empty());
    body.fix_up_img_alt();
    Ok(body)
}

struct Stack {
    stack: Vec<Element>,
}

impl Stack {
    fn new() -> Self {
        Self { stack: vec![] }
    }

    fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    fn push(&mut self, e: Element) {
        trace!("pushed {:?}", e);
        self.stack.push(e);
    }

    fn push_tag(&mut self, tag: ElementTag, loc: Location) {
        self.push(Element::new(tag).with_location(loc));
    }

    fn pop(&mut self) -> Element {
        let e = self.stack.pop().unwrap();
        trace!("popped {:?}", e);
        e
    }

    fn append_child(&mut self, child: Content) {
        trace!("appended {:?}", child);
        let mut parent = self.stack.pop().unwrap();
        parent.push_child(child);
        self.stack.push(parent);
    }

    fn append_str(&mut self, text: &str) {
        self.append_child(Content::Text(text.into()));
    }

    fn append_element(&mut self, e: Element) {
        self.append_child(Content::Elt(e));
    }
}
