//! Typeset various data types into HTML.

use base64::prelude::{Engine as _, BASE64_STANDARD};

use crate::{
    bindings::Bindings,
    diagrams::{DiagramMarkup, DotMarkup, PikchrMarkup, PlantumlMarkup, Svg},
    doc::Document,
    error::SubplotError,
    html::{Attribute, Content, Element, ElementTag, HtmlPage, Location},
    matches::{MatchedStep, PartialStep},
    resource,
    steps::ScenarioStep,
    toc::TableOfContents,
    StepSnippet,
};

// Name of standard Subplot CSS file.
const CSS: &str = "subplot.css";

// List of attributes that we don't want a typeset element to have.
const UNWANTED_ATTRS: &[&str] = &["add-newline"];

/// Return Document as an HTML page serialized into HTML text
pub fn typeset_doc(doc: &Document) -> Result<String, SubplotError> {
    let css_file = resource::read_as_string(CSS, None)
        .map_err(|e| SubplotError::CssFileNotFound(CSS.into(), e))?;

    let mut head = Element::new(crate::html::ElementTag::Head);
    let mut title = Element::new(crate::html::ElementTag::Title);
    title.push_child(crate::html::Content::Text(doc.meta().title().into()));
    head.push_child(crate::html::Content::Elt(title));

    let mut css = Element::new(ElementTag::Style);
    css.push_child(Content::Text(css_file));
    for css_file in doc.meta().css_embed() {
        css.push_child(Content::Text(css_file.into()));
    }
    head.push_child(Content::Elt(css));

    for css_url in doc.meta().css_urls() {
        let mut link = Element::new(ElementTag::Link);
        link.push_attribute(Attribute::new("rel", "stylesheet"));
        link.push_attribute(Attribute::new("type", "text/css"));
        link.push_attribute(Attribute::new("href", css_url));
        head.push_child(Content::Elt(link));
    }

    let mut body_content = Element::new(crate::html::ElementTag::Div);
    body_content.push_attribute(Attribute::new("class", "content"));
    for md in doc.markdowns().iter() {
        body_content.push_child(Content::Elt(md.root_element().clone()));
    }

    let mut body = Element::new(crate::html::ElementTag::Div);
    body.push_child(Content::Elt(meta(doc)));
    body.push_child(Content::Elt(toc(doc.toc())));
    body.push_child(Content::Elt(body_content));

    let page = HtmlPage::new(head, body);
    page.serialize().map_err(SubplotError::ParseMarkdown)
}

fn meta(doc: &Document) -> Element {
    let mut div = Element::new(ElementTag::Div);
    div.push_attribute(Attribute::new("class", "meta"));

    div.push_child(Content::Elt(title(doc.meta().title())));

    if let Some(names) = doc.meta().authors() {
        div.push_child(Content::Elt(authors(names)));
    }

    if let Some(d) = doc.meta().date() {
        div.push_child(Content::Elt(date(d)));
    }

    div
}

fn title(title: &str) -> Element {
    let mut e = Element::new(ElementTag::H1);
    e.push_attribute(Attribute::new("class", "title"));
    e.push_child(Content::Text(title.into()));
    e
}

fn authors(authors: &[String]) -> Element {
    let mut list = Element::new(ElementTag::P);
    list.push_attribute(Attribute::new("class", "authors"));
    list.push_child(Content::Text("By: ".into()));
    let mut first = true;
    for a in authors {
        if !first {
            list.push_child(Content::Text(", ".into()));
        }
        list.push_child(Content::Text(a.into()));
        first = false;
    }
    list
}

fn date(date: &str) -> Element {
    let mut e = Element::new(ElementTag::P);
    e.push_attribute(Attribute::new("class", "date"));
    e.push_child(Content::Text(date.into()));
    e
}

// Create a table of contents as an HTML element. The element is a Div
// and includes an H1 heading, and a UL list with the entries to
// headings in the document.
fn toc(toc: &TableOfContents) -> Element {
    let mut toc_element = Element::new(ElementTag::Div);
    toc_element.push_attribute(Attribute::new("class", "toc"));

    let mut heading = Element::new(ElementTag::H1);
    heading.push_child(Content::Text("Table of Contents".into()));
    toc_element.push_child(Content::Elt(heading));

    let mut stack = vec![Element::new(ElementTag::Ul)];

    for h in toc.iter() {
        let mut number = Element::new(ElementTag::Span);
        number.push_attribute(Attribute::new("class", "heading-number"));
        number.push_child(Content::Text(h.number.clone()));

        let mut content = Element::new(ElementTag::Span);
        content.push_attribute(Attribute::new("class", "heading-text"));
        content.push_child(h.content.clone());

        let mut a = Element::new(ElementTag::A);
        a.push_attribute(crate::html::Attribute::new("href", &format!("#{}", h.slug)));
        a.push_attribute(Attribute::new("class", "toc-link"));
        a.push_child(Content::Elt(number));
        a.push_child(Content::Text(" ".into()));
        a.push_child(Content::Elt(content));

        let mut li = Element::new(ElementTag::Li);
        li.push_child(Content::Elt(a));

        assert!(!stack.is_empty());
        if h.level == stack.len() {
            // Level stays the same. Append li to topmost list element.
            let mut list = stack.pop().unwrap();
            list.push_child(Content::Elt(li));
            stack.push(list);
        } else if h.level > stack.len() {
            // Level increases. Start a new list element.
            let mut list = Element::new(ElementTag::Ul);
            list.push_child(Content::Elt(li));
            stack.push(list);
        } else if stack.len() > 1 {
            // Level decreases. Append topmost element to its parent,
            // then a new heading to the same parent.
            let inner = stack.pop().unwrap();
            let mut outer = stack.pop().unwrap();
            outer.push_child(Content::Elt(inner));
            outer.push_child(Content::Elt(li));
            stack.push(outer);
        } else {
            // Something is wrong. Just append heading to topmost list on stack.
            let mut list = stack.pop().unwrap();
            list.push_child(Content::Elt(li));
            stack.push(list);
        }
    }

    while stack.len() > 1 {
        let inner = stack.pop().unwrap();
        let mut outer = stack.pop().unwrap();
        outer.push_child(Content::Elt(inner));
        stack.push(outer);
    }

    if let Some(list) = stack.pop() {
        toc_element.push_child(Content::Elt(list));
    }

    toc_element
}

/// Type set an HTML element.
pub fn typeset_element(
    e: &Element,
    template: Option<&str>,
    bindings: &Bindings,
) -> Result<Element, SubplotError> {
    let new = match e.tag() {
        ElementTag::Pre if e.has_attr("class", "scenario") => {
            typeset_scenario(e, template, bindings)
        }
        ElementTag::Pre if e.has_attr("class", "file") => file(e),
        ElementTag::Pre if e.has_attr("class", "example") => example(e),
        ElementTag::Pre if e.has_attr("class", "dot") => dot(e),
        ElementTag::Pre if e.has_attr("class", "plantuml") => plantuml(e),
        ElementTag::Pre if e.has_attr("class", "roadmap") => roadmap(e),
        ElementTag::Pre if e.has_attr("class", "pikchr") => pikchr(e),
        _ => {
            let mut new = Element::new(e.tag());
            for attr in e.all_attrs() {
                new.push_attribute(attr.clone());
            }
            for child in e.children() {
                if let Content::Elt(ce) = child {
                    new.push_child(Content::Elt(typeset_element(ce, template, bindings)?));
                } else {
                    new.push_child(child.clone());
                }
            }
            Ok(new)
        }
    };
    let mut new = new?;
    new.drop_attributes(UNWANTED_ATTRS);
    Ok(new)
}

fn typeset_scenario(
    e: &Element,
    template: Option<&str>,
    bindings: &Bindings,
) -> Result<Element, SubplotError> {
    let template = template.unwrap_or("python"); // FIXME

    let text = e.content();
    let steps = crate::steps::parse_scenario_snippet(&text, &Location::Unknown)?;

    let mut scenario = Element::new(ElementTag::Div);
    scenario.push_attribute(Attribute::new("class", "scenario"));

    for st in steps {
        let st = if let Ok(matched) = bindings.find(template, &st) {
            matched_step(&matched)
        } else {
            unmatched_step(&st)
        };
        scenario.push_child(Content::Elt(st));
    }

    Ok(scenario)
}

fn matched_step(step: &MatchedStep) -> Element {
    let parts: Vec<&PartialStep> = step.parts().collect();
    step_helper(step.kind().to_string(), &parts)
}

fn unmatched_step(step: &ScenarioStep) -> Element {
    let text = PartialStep::UncapturedText(StepSnippet::new(step.text()));
    let parts = vec![&text];
    step_helper(step.kind().to_string(), &parts)
}

fn step_helper(step_kind: String, parts: &[&PartialStep]) -> Element {
    let mut e = Element::new(ElementTag::Div);
    let mut keyword = Element::new(ElementTag::Span);
    keyword.push_attribute(Attribute::new("class", "keyword"));
    keyword.push_child(Content::Text(step_kind));
    keyword.push_child(Content::Text(" ".into()));
    e.push_child(Content::Elt(keyword));
    for part in parts {
        match part {
            PartialStep::UncapturedText(snippet) => {
                let text = snippet.text();
                if !text.trim().is_empty() {
                    let mut estep = Element::new(ElementTag::Span);
                    estep.push_attribute(Attribute::new("class", "uncaptured"));
                    estep.push_child(Content::Text(text.into()));
                    e.push_child(Content::Elt(estep));
                }
            }
            PartialStep::CapturedText {
                name: _,
                text,
                kind,
            } => {
                if !text.trim().is_empty() {
                    let mut estep = Element::new(ElementTag::Span);
                    let class = format!("capture-{}", kind.as_str());
                    estep.push_attribute(Attribute::new("class", &class));
                    estep.push_child(Content::Text(text.into()));
                    e.push_child(Content::Elt(estep));
                }
            }
        }
    }
    e
}

fn file(e: &Element) -> Result<Element, SubplotError> {
    Ok(e.clone()) // FIXME
}

fn example(e: &Element) -> Result<Element, SubplotError> {
    Ok(e.clone()) // FIXME
}

fn dot(e: &Element) -> Result<Element, SubplotError> {
    let dot = e.content();
    let svg = DotMarkup::new(&dot).as_svg()?;
    Ok(svg_to_element(svg, "Dot diagram"))
}

fn plantuml(e: &Element) -> Result<Element, SubplotError> {
    let markup = e.content();
    let svg = PlantumlMarkup::new(&markup).as_svg()?;
    Ok(svg_to_element(svg, "UML diagram"))
}

fn pikchr(e: &Element) -> Result<Element, SubplotError> {
    let markup = e.content();
    let svg = PikchrMarkup::new(&markup, None).as_svg()?;
    Ok(svg_to_element(svg, "Pikchr diagram"))
}

fn roadmap(e: &Element) -> Result<Element, SubplotError> {
    const WIDTH: usize = 50;

    let yaml = e.content();
    let roadmap = roadmap::from_yaml(&yaml)?;
    let dot = roadmap.format_as_dot(WIDTH)?;
    let svg = DotMarkup::new(&dot).as_svg()?;
    Ok(svg_to_element(svg, "Road map"))
}

fn svg_to_element(svg: Svg, alt: &str) -> Element {
    let url = svg_as_data_url(svg);
    let img = html_img(&url, alt);
    html_p(vec![Content::Elt(img)])
}

fn svg_as_data_url(svg: Svg) -> String {
    let svg = BASE64_STANDARD.encode(svg.data());
    format!("data:image/svg+xml;base64,{svg}")
}

fn html_p(children: Vec<Content>) -> Element {
    let mut new = Element::new(ElementTag::P);
    for child in children {
        new.push_child(child);
    }
    new
}

fn html_img(src: &str, alt: &str) -> Element {
    let mut new = Element::new(ElementTag::Img);
    new.push_attribute(Attribute::new("src", src));
    new.push_attribute(Attribute::new("alt", alt));
    new
}
