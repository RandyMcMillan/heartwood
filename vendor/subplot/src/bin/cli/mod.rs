//! CLI Functionality abstractions

#![allow(unused)]

use anyhow::Result;
use clap::ValueEnum;
use log::trace;
use serde::Serialize;
use std::fmt::Debug;
use std::path::Path;
use std::str::FromStr;
use std::{collections::HashMap, convert::TryFrom};
use subplot::{Document, EmbeddedFile, Style, SubplotError};

pub fn extract_file<'a>(doc: &'a Document, filename: &str) -> Result<&'a EmbeddedFile> {
    for file in doc.embedded_files() {
        if file.filename() == filename {
            return Ok(file);
        }
    }
    Err(SubplotError::EmbeddedFileNotFound(filename.to_owned()).into())
}

#[derive(Serialize)]
pub struct Metadata {
    sources: Vec<String>,
    title: String,
    binding_files: Vec<String>,
    impls: HashMap<String, Vec<String>>,
    scenarios: Vec<String>,
    files: Vec<String>,
}

impl TryFrom<&mut Document> for Metadata {
    type Error = subplot::SubplotError;
    fn try_from(doc: &mut Document) -> std::result::Result<Self, Self::Error> {
        let mut sources: Vec<_> = doc
            .sources(None)
            .into_iter()
            .map(|p| filename(Some(&p)))
            .collect();
        sources.sort_unstable();
        let title = doc.meta().title().to_owned();
        let mut binding_files: Vec<_> = doc
            .meta()
            .bindings_filenames()
            .into_iter()
            .map(|p| filename(Some(p)))
            .collect();
        binding_files.sort_unstable();
        let impls: HashMap<_, _> = doc
            .meta()
            .templates()
            .map(|template| {
                let mut filenames: Vec<_> = doc
                    .meta()
                    .document_impl(template)
                    .unwrap()
                    .functions_filenames()
                    .map(|p| filename(Some(p)))
                    .collect();
                filenames.sort_unstable();
                (template.to_string(), filenames)
            })
            .collect();
        let mut scenarios: Vec<_> = doc
            .scenarios()?
            .into_iter()
            .map(|s| s.title().to_owned())
            .collect();
        scenarios.sort_unstable();
        let mut files: Vec<_> = doc
            .embedded_files()
            .iter()
            .map(|f| f.filename().to_owned())
            .collect();
        files.sort_unstable();
        Ok(Self {
            sources,
            title,
            binding_files,
            impls,
            scenarios,
            files,
        })
    }
}

impl Metadata {
    fn write_list(v: &[String], prefix: &str) {
        v.iter().for_each(|entry| println!("{prefix}: {entry}"))
    }

    pub fn write_out(&self) {
        Self::write_list(&self.sources, "source");
        println!("title: {}", self.title);
        Self::write_list(&self.binding_files, "bindings");
        let templates: Vec<String> = self.impls.keys().map(String::from).collect();
        Self::write_list(&templates, "templates");
        for (template, filenames) in self.impls.iter() {
            Self::write_list(filenames, &format!("functions[{template}]"));
        }
        Self::write_list(&self.files, "file");
        Self::write_list(&self.scenarios, "scenario");
    }
}

fn filename(name: Option<&Path>) -> String {
    let path = match name {
        None => return "".to_string(),
        Some(x) => x,
    };
    match path.to_str() {
        None => "non-UTF8 filename".to_string(),
        Some(x) => x.to_string(),
    }
}

#[derive(Debug, ValueEnum, Clone, Copy)]
pub enum OutputFormat {
    Plain,
    Json,
}

impl FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_ref() {
            "plain" => Ok(OutputFormat::Plain),
            "json" => Ok(OutputFormat::Json),
            _ => Err(format!("Unknown output format: `{s}`")),
        }
    }
}
