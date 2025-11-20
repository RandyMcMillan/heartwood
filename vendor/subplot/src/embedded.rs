use serde::{Deserialize, Serialize};

/// A data file embedded in the document.
#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct EmbeddedFile {
    filename: String,
    contents: String,
}

impl EmbeddedFile {
    /// Create a new data file, with a name and contents.
    pub fn new(filename: String, contents: String) -> EmbeddedFile {
        EmbeddedFile { filename, contents }
    }

    /// Return name of embedded file.
    pub fn filename(&self) -> &str {
        &self.filename
    }

    /// Return contents of embedded file.
    pub fn contents(&self) -> &str {
        &self.contents
    }
}

/// A collection of data files embedded in document.
#[derive(Debug, Default, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct EmbeddedFiles {
    files: Vec<EmbeddedFile>,
}

impl EmbeddedFiles {
    /// Return slice of all data files.
    pub fn files(&self) -> &[EmbeddedFile] {
        &self.files
    }

    /// Append a new data file.
    pub fn push(&mut self, file: EmbeddedFile) {
        self.files.push(file);
    }
}
