/// Typesetting style configuration for documents.
///
/// This contains settings that affect how the document abstract
/// syntax tree is modified during typesetting.
#[derive(Clone, Debug, Default)]
pub struct Style {
    links_as_notes: bool,
}

impl Style {
    /// Should hyperlinks in the document be rendered as footnotes or endnotes?
    ///
    /// A link is like the HTML `<a>` element. The choice of footnote
    /// versus endnote is made by the typesetting backend. HTML uses
    /// endnotes, a paged media like PDF would use footnotes.
    pub fn links_as_notes(&self) -> bool {
        self.links_as_notes
    }

    /// Make links be typeset as notes.
    ///
    /// Default is that they are typeset as links. This can't be
    /// unset.
    pub fn typeset_links_as_notes(&mut self) {
        self.links_as_notes = true
    }
}
