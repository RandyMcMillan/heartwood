//! Subplot top level binary
//!

use anyhow::Result;

use log::{debug, error, info, trace, warn};
use subplot::{
    codegen, load_document, resource, Binding, Bindings, Document, EmbeddedFile, MarkupOpts, Style,
    SubplotError, Warnings,
};
use time::{format_description::FormatItem, macros::format_description, OffsetDateTime};

use clap::{CommandFactory, FromArgMatches, Parser};
use std::convert::TryFrom;
use std::fs::{self, write};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::time::UNIX_EPOCH;

mod cli;

use git_testament::*;

git_testament!(VERSION);

#[derive(Debug, Parser)]
struct Toplevel {
    #[clap(flatten)]
    resources: resource::ResourceOpts,

    #[clap(flatten)]
    markup: MarkupOpts,

    #[clap(subcommand)]
    command: Cmd,
}

impl Toplevel {
    fn run(&self) -> Result<()> {
        trace!("Toplevel: {:?}", self);
        self.command.run()
    }

    fn handle_special_args(&self) {
        let doc_path = self.command.doc_path();
        self.resources.handle(doc_path);
        self.markup.handle();
    }
}

#[derive(Debug, Parser)]
enum Cmd {
    Extract(Extract),
    Metadata(Metadata),
    Docgen(Docgen),
    Codegen(Codegen),
    #[clap(hide = true)]
    Resources(Resources),
    #[clap(hide = true)]
    Libdocgen(Libdocgen),
}

impl Cmd {
    fn run(&self) -> Result<()> {
        match self {
            Cmd::Extract(e) => e.run(),
            Cmd::Metadata(m) => m.run(),
            Cmd::Docgen(d) => d.run(),
            Cmd::Codegen(c) => c.run(),
            Cmd::Resources(r) => r.run(),
            Cmd::Libdocgen(r) => r.run(),
        }
    }

    fn doc_path(&self) -> Option<&Path> {
        match self {
            Cmd::Extract(e) => e.doc_path(),
            Cmd::Metadata(m) => m.doc_path(),
            Cmd::Docgen(d) => d.doc_path(),
            Cmd::Codegen(c) => c.doc_path(),
            Cmd::Resources(r) => r.doc_path(),
            Cmd::Libdocgen(r) => r.doc_path(),
        }
    }
}

fn long_version() -> Result<String> {
    use std::fmt::Write as _;
    let mut ret = String::new();
    writeln!(ret, "{}", render_testament!(VERSION))?;
    writeln!(ret, "Crate version: {}", env!("CARGO_PKG_VERSION"))?;
    if let Some(branch) = VERSION.branch_name {
        writeln!(ret, "Built from branch: {branch}")?;
    } else {
        writeln!(ret, "Branch information is missing.")?;
    }
    writeln!(ret, "Commit info: {}", VERSION.commit)?;
    if VERSION.modifications.is_empty() {
        writeln!(ret, "Working tree is clean")?;
    } else {
        use GitModification::*;
        for fmod in VERSION.modifications {
            match fmod {
                Added(f) => writeln!(ret, "Added: {}", String::from_utf8_lossy(f))?,
                Removed(f) => writeln!(ret, "Removed: {}", String::from_utf8_lossy(f))?,
                Modified(f) => writeln!(ret, "Modified: {}", String::from_utf8_lossy(f))?,
                Untracked(f) => writeln!(ret, "Untracked: {}", String::from_utf8_lossy(f))?,
            }
        }
    }
    Ok(ret)
}

#[derive(Debug, Parser)]
/// Examine embedded resources built into Subplot
struct Resources {}

impl Resources {
    fn doc_path(&self) -> Option<&Path> {
        None
    }
    fn run(&self) -> Result<()> {
        for (name, bytes) in subplot::resource::embedded_files() {
            println!("{} {} bytes", name, bytes.len());
        }
        Ok(())
    }
}
#[derive(Debug, Parser)]
/// Extract embedded files from a subplot document
///
/// If no embedded filenames are provided, this will
/// extract all embedded files.  if the output directory
/// is not specified then this will extract to the current directory.
struct Extract {
    /// Allow warnings in document?
    #[clap(long)]
    merciful: bool,

    /// Directory to write extracted files to
    #[clap(name = "DIR", long = "directory", short = 'd', default_value = ".")]
    directory: PathBuf,

    /// Don't actually write the files out
    #[clap(long)]
    dry_run: bool,

    /// Input subplot document filename
    filename: PathBuf,

    /// Names of embedded files to be extracted.
    embedded: Vec<String>,
}

impl Extract {
    fn doc_path(&self) -> Option<&Path> {
        self.filename.parent()
    }

    fn run(&self) -> Result<()> {
        let doc = load_linted_doc(&self.filename, Style::default(), None, self.merciful)?;

        let files: Vec<&EmbeddedFile> = if self.embedded.is_empty() {
            doc.embedded_files()
                .iter()
                .map(Result::Ok)
                .collect::<Result<Vec<_>>>()
        } else {
            self.embedded
                .iter()
                .map(|filename| cli::extract_file(&doc, filename))
                .collect::<Result<Vec<_>>>()
        }?;

        for f in files {
            let outfile = self.directory.join(f.filename());
            if self.dry_run {
                println!("Would write out {}", outfile.display());
            } else {
                write(outfile, f.contents())?
            }
        }

        Ok(())
    }
}

#[derive(Debug, Parser)]
/// Extract metadata about a document
///
/// Load and process a subplot document, extracting various metadata about the
/// document.  This can then render that either as a plain text report for humans,
/// or as a JSON object for further scripted processing.
struct Metadata {
    /// Allow warnings in document?
    #[clap(long)]
    merciful: bool,

    /// Form that you want the output to take
    #[clap(short = 'o', default_value = "plain")]
    output_format: cli::OutputFormat,

    /// Input subplot document filename
    filename: PathBuf,
}

impl Metadata {
    fn doc_path(&self) -> Option<&Path> {
        self.filename.parent()
    }

    fn run(&self) -> Result<()> {
        let mut doc = load_linted_doc(&self.filename, Style::default(), None, self.merciful)?;
        let meta = cli::Metadata::try_from(&mut doc)?;
        match self.output_format {
            cli::OutputFormat::Plain => meta.write_out(),
            cli::OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&meta)?),
        }
        Ok(())
    }
}

#[derive(Debug, Parser)]
/// Typeset subplot document
///
/// Process a subplot document and typeset it.
struct Docgen {
    /// Allow warnings in document?
    #[clap(long)]
    merciful: bool,

    /// The template to use from the document.
    ///
    /// If not specified, subplot will try and find a unique template name from the document
    #[clap(name = "TEMPLATE", long = "template", short = 't')]
    template: Option<String>,

    // Input Subplot document
    input: PathBuf,

    // Output document filename
    #[clap(name = "FILE", long = "output", short = 'o')]
    output: PathBuf,

    // Set date.
    #[clap(name = "DATE", long = "date")]
    date: Option<String>,
}

impl Docgen {
    fn doc_path(&self) -> Option<&Path> {
        self.input.parent()
    }

    fn run(&self) -> Result<()> {
        let style = Style::default();
        let mut doc = load_linted_doc(&self.input, style, self.template.as_deref(), self.merciful)?;

        // Metadata date from command line or file mtime. However, we
        // can't set it directly, since we don't want to override the date
        // in the actual document, if given, so we only set
        // user-provided-date. Our parsing code will use that if date is
        // not document metadata.
        let date = if let Some(date) = self.date.clone() {
            date
        } else if let Some(date) = doc.meta().date() {
            date.to_string()
        } else {
            let mut newest = None;
            let basedir = if let Some(basedir) = self.input.parent() {
                basedir.to_path_buf()
            } else {
                return Err(SubplotError::BasedirError(self.input.clone()).into());
            };
            for filename in doc.meta().markdown_filenames() {
                let filename = basedir.join(filename);
                let mtime = Self::mtime(&filename)?;
                if let Some(so_far) = newest {
                    if mtime > so_far {
                        newest = Some(mtime);
                    }
                } else {
                    newest = Some(mtime);
                }
            }
            Self::mtime_formatted(newest.unwrap())
        };

        doc.typeset(&mut Warnings::default(), self.template.as_deref())?;
        std::fs::write(&self.output, doc.to_html(&date)?)
            .map_err(|e| SubplotError::WriteFile(self.output.clone(), e))?;

        Ok(())
    }

    fn mtime(filename: &Path) -> Result<(u64, u32)> {
        let mtime = fs::metadata(filename)
            .map_err(|e| SubplotError::InputFileUnreadable(filename.into(), e))?
            .modified()
            .map_err(|e| SubplotError::InputFileMtime(filename.into(), e))?;
        let mtime = mtime.duration_since(UNIX_EPOCH)?;
        Ok((mtime.as_secs(), mtime.subsec_nanos()))
    }

    fn mtime_formatted(mtime: (u64, u32)) -> String {
        const DATE_FORMAT: &[FormatItem<'_>] =
            format_description!("[year]-[month]-[day] [hour]:[minute]");

        let secs: i64 = format!("{}", mtime.0).parse().unwrap_or(0);
        let time = OffsetDateTime::from_unix_timestamp(secs).unwrap();
        time.format(DATE_FORMAT).unwrap()
    }
}

#[derive(Debug, Parser)]
/// Generate test suites from Subplot documents
///
/// This reads a subplot document, extracts the scenarios, and writes out a test
/// program capable of running the scenarios in the subplot document.
struct Codegen {
    /// The template to use from the document.
    ///
    /// If not specified, subplot will try and find a unique template name from the document
    #[clap(name = "TEMPLATE", long = "template", short = 't')]
    template: Option<String>,

    /// Input filename.
    filename: PathBuf,

    /// Write generated test program to this file.
    #[clap(long, short, help = "Writes generated test program to FILE")]
    output: PathBuf,

    /// Run the generated test program after writing it?
    #[clap(long, short, help = "Runs generated test program")]
    run: bool,
}

impl Codegen {
    fn doc_path(&self) -> Option<&Path> {
        self.filename.parent()
    }

    fn run(&self) -> Result<()> {
        debug!("codegen starts");
        let output = codegen(&self.filename, &self.output, self.template.as_deref())?;
        if self.run {
            let spec = output
                .doc
                .meta()
                .document_impl(&output.template)
                .unwrap()
                .spec();
            let run = match spec.run() {
                None => {
                    error!(
                        "Template {} does not specify how to run suites",
                        spec.template_filename().display()
                    );
                    std::process::exit(1);
                }
                Some(x) => x,
            };

            let status = Command::new(run).arg(&self.output).status()?;
            if !status.success() {
                error!("Test suite failed!");
                std::process::exit(2);
            }
        }
        debug!("codegen ends successfully");
        Ok(())
    }
}

#[derive(Debug, Parser)]
/// Generate test suites from Subplot documents
///
/// This reads a subplot document, extracts the scenarios, and writes out a test
/// program capable of running the scenarios in the subplot document.
struct Libdocgen {
    // Bindings file to read.
    input: PathBuf,

    // Output document filename
    #[clap(name = "FILE", long = "output", short = 'o')]
    output: PathBuf,

    /// The template to use from the document.
    ///
    /// If not specified, subplot will try and find a unique template name from the document
    #[clap(name = "TEMPLATE", long = "template", short = 't')]
    template: Option<String>,

    /// Be merciful by allowing bindings to not have documentation.
    #[clap(long)]
    merciful: bool,
}

impl Libdocgen {
    fn doc_path(&self) -> Option<&Path> {
        None
    }

    fn run(&self) -> Result<()> {
        debug!("libdocgen starts");

        let mut bindings = Bindings::new();
        bindings.add_from_file(&self.input, None)?;
        // println!("{:#?}", bindings);

        let mut doc = LibDoc::new(&self.input);
        for b in bindings.bindings() {
            // println!("{} {}", b.kind(), b.pattern());
            doc.push_binding(b);
        }

        std::fs::write(&self.output, doc.to_markdown(self.merciful)?)?;

        debug!("libdogen ends successfully");
        Ok(())
    }
}

struct LibDoc {
    filename: PathBuf,
    bindings: Vec<Binding>,
}

impl LibDoc {
    fn new(filename: &Path) -> Self {
        Self {
            filename: filename.into(),
            bindings: vec![],
        }
    }

    fn push_binding(&mut self, binding: &Binding) {
        self.bindings.push(binding.clone());
    }

    fn to_markdown(&self, merciful: bool) -> Result<String> {
        let mut md = String::new();
        md.push_str(&format!("# Library `{}`\n\n", self.filename.display()));
        for b in self.bindings.iter() {
            md.push_str(&format!("\n## {} `{}`\n", b.kind(), b.pattern()));
            if let Some(doc) = b.doc() {
                md.push_str(&format!("\n{}\n", doc));
            } else if !merciful {
                return Err(SubplotError::NoBindingDoc(
                    self.filename.clone(),
                    b.kind(),
                    b.pattern().into(),
                )
                .into());
            }
            if b.types().count() > 0 {
                md.push_str("\nCaptures:\n\n");
                for (name, cap_type) in b.types() {
                    md.push_str(&format!("- `{}`: {}\n", name, cap_type.as_str()));
                }
            }
        }
        Ok(md)
    }
}

fn load_linted_doc(
    filename: &Path,
    style: Style,
    template: Option<&str>,
    merciful: bool,
) -> Result<Document, SubplotError> {
    let doc = load_document(filename, style, None)?;
    trace!("Got doc, now linting it");
    doc.lint()?;
    trace!("Doc linted ok");

    let meta = doc.meta();
    trace!("Looking for template, meta={:#?}", meta);

    // Get template as given to use (from command line), or from
    // document, or else default to the empty string.
    let template = if let Some(t) = template {
        t
    } else if let Ok(t) = doc.template() {
        t
    } else {
        ""
    };
    let template = template.to_string();
    trace!("Template: {:#?}", template);
    let mut warnings = Warnings::default();
    doc.check_bindings(&mut warnings)?;
    doc.check_named_code_blocks_have_appropriate_class(&mut warnings)?;
    doc.check_named_files_exist(&template, &mut warnings)?;
    if !template.is_empty() {
        // We have a template, let's check we have implementations
        doc.check_matched_steps_have_impl(&template, &mut warnings);
    } else {
        trace!("No template found, so cannot check impl presence");
    }
    doc.check_embedded_files_are_used(&template, &mut warnings)?;

    for w in warnings.warnings() {
        warn!("{}", w);
    }

    if !warnings.is_empty() && !merciful {
        return Err(SubplotError::Warnings(warnings.len()));
    }

    Ok(doc)
}

fn real_main() {
    info!("Starting Subplot");
    let argparser = Toplevel::command();
    let version = long_version().unwrap();
    let argparser = argparser.long_version(version);
    let args = argparser.get_matches();
    let args = Toplevel::from_arg_matches(&args).unwrap();
    args.handle_special_args();
    match args.run() {
        Ok(_) => {
            info!("Subplot finished successfully");
        }
        Err(e) => {
            error!("{}", e);
            let mut e = e.source();
            while let Some(source) = e {
                error!("caused by: {}", source);
                e = source.source();
            }
            process::exit(1);
        }
    }
}

fn main() {
    let env = env_logger::Env::new()
        .filter_or("SUBPLOT_LOG", "info")
        .write_style("SUBPLOT_LOG_STYLE");
    env_logger::Builder::from_env(env)
        .format_timestamp(None)
        .format(|buf, record| writeln!(buf, "{}: {}", record.level(), record.args()))
        .init();
    real_main();
}
