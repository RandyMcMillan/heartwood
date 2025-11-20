//! Store information needed to build a table of contents for a
//! document.

use std::cmp::Ordering;
use std::collections::HashSet;

use slug::slugify;

use crate::html::{as_plain_text, Content};

// Compute a short unique name ("slug") for a heading in a document.
#[derive(Debug, Default)]
struct SlugFactory {
    known: HashSet<String>,
}

impl SlugFactory {
    /// Compute a slug for a given heading text.
    fn slug(&mut self, text: &str) -> Result<String, SlugError> {
        let simple = slugify(text);
        let slug = self.unique_slug(&simple)?;
        self.known.insert(slug.clone());
        Ok(slug)
    }

    fn given(&mut self, slug: &str) {
        self.known.insert(slug.into());
    }

    fn unique_slug(&mut self, simple: &str) -> Result<String, SlugError> {
        const MAX_TRIES: usize = 1000;

        if self.known.contains(simple) {
            for i in 2..MAX_TRIES {
                let slug = format!("{simple}{i}");
                if !self.known.contains(&slug) {
                    return Ok(slug);
                }
            }
            Err(SlugError::Unique(MAX_TRIES, simple.into()))
        } else {
            Ok(simple.into())
        }
    }
}

/// Possible errors from creating a slug.
#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum SlugError {
    /// Can't create a slug. This is probably due to too many headings
    /// that start with the same words.
    #[error("failed to generate a unique slug in {0} tries for {1:?}")]
    Unique(usize, String),
}

#[cfg(test)]
mod test_slugs {
    use super::SlugFactory;

    #[test]
    fn short_and_simple() {
        let mut slugs = SlugFactory::default();
        assert_eq!(slugs.slug("Foo"), Ok("foo".into()));
    }

    #[test]
    fn unique_for_identical_simple_headings() {
        let mut slugs = SlugFactory::default();
        assert_ne!(slugs.slug("Foo"), slugs.slug("Foo"));
    }
}

// Number all headings in a document.
//
// Headings are numbered sequentially. The first heading is 1. A new
// heading increments the least significant component of a heading one
// the same level as the new one. Thus, the second heading is 2, or
// 1.1, if it's a subheading.
#[derive(Debug, Default)]
struct HeadingNumberer {
    prev: Vec<usize>,
}

impl HeadingNumberer {
    // Pick number a new heading: 1, 1.1, 1.2, 1.3, 2, 2.1, etc.
    // There can be any number of levels..
    fn number(&mut self, level: usize) -> String {
        match level.cmp(&self.prev.len()) {
            Ordering::Equal => {
                if let Some(n) = self.prev.pop() {
                    self.prev.push(n + 1);
                } else {
                    self.prev.push(1);
                }
            }
            Ordering::Greater => {
                self.prev.push(1);
            }
            Ordering::Less => {
                assert!(!self.prev.is_empty());
                self.prev.pop();
                if let Some(n) = self.prev.pop() {
                    self.prev.push(n + 1);
                } else {
                    self.prev.push(1);
                }
            }
        }

        let mut s = String::new();
        for i in self.prev.iter() {
            if !s.is_empty() {
                s.push('.');
            }
            s.push_str(&i.to_string());
        }
        s
    }
}

#[cfg(test)]
mod test_numberer {
    use super::HeadingNumberer;

    #[test]
    fn numbering() {
        let mut n = HeadingNumberer::default();
        assert_eq!(n.number(1), "1");
        assert_eq!(n.number(2), "1.1");
        assert_eq!(n.number(1), "2");
        assert_eq!(n.number(2), "2.1");
    }
}

/// A table of contents.
#[derive(Debug, Default)]
pub struct TableOfContents {
    slugs: SlugFactory,
    numbers: HeadingNumberer,
    headings: Vec<Heading>,
}

impl TableOfContents {
    /// Push a new heading, returning its unique slug. Use the slug
    /// given, if any.
    pub fn push_heading(
        &mut self,
        level: usize,
        content: &Content,
        slug: Option<&str>,
    ) -> Result<Heading, ToCError> {
        let text = as_plain_text(&[content.clone()]);
        let slug = if let Some(slug) = slug {
            self.slugs.given(slug);
            slug.to_string()
        } else {
            self.slugs.slug(&text)?
        };

        let number = self.numbers.number(level);
        let heading = Heading {
            level,
            slug: slug.clone(),
            number,
            content: content.clone(),
        };
        self.headings.push(heading.clone());
        Ok(heading)
    }

    /// Iterate over slugs for all headings.
    pub fn iter(&self) -> impl Iterator<Item = &Heading> {
        self.headings.iter()
    }
}

/// Possible errors for table of contents.
#[derive(Debug, thiserror::Error)]
pub enum ToCError {
    /// Error from generating a slug.
    #[error(transparent)]
    Slug(#[from] SlugError),
}

/// Represent one heading in a table of contents.
#[derive(Debug, Clone)]
pub struct Heading {
    /// The level of the heading.
    pub level: usize,

    /// The slug identifier for the heading.
    pub slug: String,

    /// The number of the heading.
    pub number: String,

    /// The HTML content of the heading.
    pub content: Content,
}

impl PartialEq for Heading {
    fn eq(&self, other: &Self) -> bool {
        self.level == other.level && self.slug == other.slug && self.number == other.number
    }
}

impl Eq for &Heading {}

#[cfg(test)]
mod test_toc {
    use crate::html::Content;

    use super::{Heading, TableOfContents};

    #[test]
    fn iterate() {
        let mut toc = TableOfContents::default();
        toc.push_heading(1, &Content::Text("Foo".into()), None)
            .unwrap();
        toc.push_heading(1, &Content::Text("Bar".into()), None)
            .unwrap();
        let expected_foo = Heading {
            level: 1,
            slug: "foo".into(),
            number: "1".into(),
            content: Content::Text("Foo".into()),
        };
        let expected_bar = Heading {
            level: 1,
            slug: "bar".into(),
            number: "2".into(),
            content: Content::Text("Bar".into()),
        };
        let actual: Vec<&Heading> = toc.iter().collect();
        assert_eq!(actual, vec![&expected_foo, &expected_bar]);
    }

    #[test]
    fn uses_given_slug() {
        let mut toc = TableOfContents::default();
        toc.push_heading(1, &Content::Text("Foo".into()), Some("foo"))
            .unwrap();
        toc.push_heading(1, &Content::Text("Foo".into()), None)
            .unwrap();
        let expected_1 = Heading {
            level: 1,
            slug: "foo".into(),
            number: "1".into(),
            content: Content::Text("Foo".into()),
        };
        let expected_2 = Heading {
            level: 1,
            slug: "foo2".into(),
            number: "2".into(),
            content: Content::Text("Foo".into()),
        };
        let actual: Vec<&Heading> = toc.iter().collect();
        assert_eq!(actual, vec![&expected_1, &expected_2]);
    }
}
