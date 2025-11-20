use crate::SubplotError;

use std::env;
use std::ffi::OsString;
use std::io::prelude::*;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Mutex;

use clap::Parser;
use lazy_static::lazy_static;
// Resources used to configure paths for dot, plantuml.jar, and friends

#[allow(missing_docs)]
#[derive(Debug, Parser)]
pub struct MarkupOpts {
    #[clap(
        long = "dot",
        help = "Path to the `dot` binary.",
        name = "DOTPATH",
        env = "SUBPLOT_DOT_PATH"
    )]
    dot_path: Option<PathBuf>,
    #[clap(
        long = "plantuml-jar",
        help = "Path to the `plantuml.jar` file.",
        name = "PLANTUMLJARPATH",
        env = "SUBPLOT_PLANTUML_JAR_PATH"
    )]
    plantuml_jar_path: Option<PathBuf>,
    #[clap(
        long = "java",
        help = "Path to Java executable (note, effectively overrides JAVA_HOME if set to an absolute path)",
        name = "JAVA_PATH",
        env = "SUBPLOT_JAVA_PATH"
    )]
    java_path: Option<PathBuf>,
}

impl MarkupOpts {
    /// Handle CLI arguments and environment variables for markup binaries
    pub fn handle(&self) {
        if let Some(dotpath) = &self.dot_path {
            DOT_PATH.lock().unwrap().clone_from(dotpath);
        }
        if let Some(plantuml_path) = &self.plantuml_jar_path {
            PLANTUML_JAR_PATH.lock().unwrap().clone_from(plantuml_path);
        }
        if let Some(java_path) = &self.java_path {
            JAVA_PATH.lock().unwrap().clone_from(java_path);
        }
    }
}

lazy_static! {
    static ref DOT_PATH: Mutex<PathBuf> = Mutex::new(env!("BUILTIN_DOT_PATH").into());
    static ref PLANTUML_JAR_PATH: Mutex<PathBuf> =
        Mutex::new(env!("BUILTIN_PLANTUML_JAR_PATH").into());
    static ref JAVA_PATH: Mutex<PathBuf> = Mutex::new(env!("BUILTIN_JAVA_PATH").into());
}

/// An SVG image.
///
/// SVG images are vector images, but we only need to treat them as
/// opaque blobs of bytes, so we don't try to represent them in any
/// other way.
pub struct Svg {
    data: Vec<u8>,
}

impl Svg {
    fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    /// Return slice of the bytes of the image.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Number of bytes in the binary representation of the image.
    #[allow(clippy::len_without_is_empty)] // is-empty doesn't make sense
    pub fn len(&self) -> usize {
        self.data.len()
    }
}

/// A code block with markup for a diagram.
///
/// The code block will be converted to an SVG image using an external
/// filter such as Graphviz dot or plantuml. SVG is the chosen image
/// format as it's suitable for all kinds of output formats from
/// typesetting.
///
/// This trait defines the interface for different kinds of markup
/// conversions. There's only one function that needs to be defined
/// for the trait.
pub trait DiagramMarkup {
    /// Convert the markup into an SVG.
    fn as_svg(&self) -> Result<Svg, SubplotError>;
}

/// A code block with pikchr markup.
///
/// ~~~~
/// use subplot::{DiagramMarkup, PikchrMarkup};
/// let markup = r#"line; box "Hello," "World!"; arrow"#;
/// let svg = PikchrMarkup::new(markup, None).as_svg().unwrap();
/// assert!(svg.len() > 0);
/// ~~~~
pub struct PikchrMarkup {
    markup: String,
    class: Option<String>,
}

impl PikchrMarkup {
    /// Create a new Pikchr Markup holder
    pub fn new(markup: &str, class: Option<&str>) -> PikchrMarkup {
        PikchrMarkup {
            markup: markup.to_owned(),
            class: class.map(str::to_owned),
        }
    }
}

impl DiagramMarkup for PikchrMarkup {
    fn as_svg(&self) -> Result<Svg, SubplotError> {
        let mut flags = pikchr::PikchrFlags::default();
        flags.generate_plain_errors();
        let image = pikchr::Pikchr::render(&self.markup, self.class.as_deref(), flags)
            .map_err(SubplotError::PikchrRenderError)?;
        Ok(Svg::new(image.as_bytes().to_vec()))
    }
}

/// A code block with Dot markup.
///
/// ~~~~
/// use subplot::{DiagramMarkup, DotMarkup};
/// let markup = r#"digraph "foo" { a -> b }"#;
/// let svg = DotMarkup::new(&markup).as_svg().unwrap();
/// assert!(svg.len() > 0);
/// ~~~~
pub struct DotMarkup {
    markup: String,
}

impl DotMarkup {
    /// Create a new DotMarkup.
    pub fn new(markup: &str) -> DotMarkup {
        DotMarkup {
            markup: markup.to_owned(),
        }
    }
}

impl DiagramMarkup for DotMarkup {
    fn as_svg(&self) -> Result<Svg, SubplotError> {
        let path = DOT_PATH.lock().unwrap().clone();
        let mut child = Command::new(&path)
            .arg("-Tsvg")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|err| SubplotError::Spawn(path.clone(), err))?;
        if let Some(stdin) = child.stdin.as_mut() {
            stdin
                .write_all(self.markup.as_bytes())
                .map_err(SubplotError::WriteToChild)?;
            let output = child
                .wait_with_output()
                .map_err(SubplotError::WaitForChild)?;
            if output.status.success() {
                Ok(Svg::new(output.stdout))
            } else {
                Err(SubplotError::child_failed("dot", &output))
            }
        } else {
            Err(SubplotError::ChildNoStdin)
        }
    }
}

/// A code block with PlantUML markup.
///
/// ~~~~
/// use subplot::{DiagramMarkup, PlantumlMarkup};
/// let markup = "@startuml\nAlice -> Bob\n@enduml";
/// let svg = PlantumlMarkup::new(&markup).as_svg().unwrap();
/// assert!(svg.len() > 0);
/// ~~~~
pub struct PlantumlMarkup {
    markup: String,
}

impl PlantumlMarkup {
    /// Create a new PlantumlMarkup.
    pub fn new(markup: &str) -> PlantumlMarkup {
        PlantumlMarkup {
            markup: markup.to_owned(),
        }
    }

    // If JAVA_HOME is set, and PATH is set, then:
    // Check if JAVA_HOME/bin is in PATH, if not, prepend it and return a new
    // PATH
    fn build_java_path() -> Option<OsString> {
        let java_home = env::var_os("JAVA_HOME")?;
        let cur_path = env::var_os("PATH")?;
        let cur_path: Vec<_> = env::split_paths(&cur_path).collect();
        let java_home = PathBuf::from(java_home);
        let java_bin = java_home.join("bin");
        if cur_path.iter().any(|v| v.as_os_str() == java_bin) {
            // No need to add JAVA_HOME/bin it's already on-path
            return None;
        }
        env::join_paths(Some(java_bin).iter().chain(cur_path.iter())).ok()
    }
}

impl DiagramMarkup for PlantumlMarkup {
    fn as_svg(&self) -> Result<Svg, SubplotError> {
        let path = JAVA_PATH.lock().unwrap().clone();
        let mut cmd = Command::new(&path);
        cmd.arg("-Djava.awt.headless=true")
            .arg("-jar")
            .arg(PLANTUML_JAR_PATH.lock().unwrap().clone())
            .arg("--")
            .arg("-pipe")
            .arg("-tsvg")
            .arg("-v")
            .arg("-graphvizdot")
            .arg(DOT_PATH.lock().unwrap().clone())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(path) = Self::build_java_path() {
            cmd.env("PATH", path);
        }
        let mut child = cmd
            .spawn()
            .map_err(|err| SubplotError::Spawn(path.clone(), err))?;
        if let Some(stdin) = child.stdin.as_mut() {
            stdin
                .write_all(self.markup.as_bytes())
                .map_err(SubplotError::WriteToChild)?;
            let output = child
                .wait_with_output()
                .map_err(SubplotError::WaitForChild)?;
            if output.status.success() {
                Ok(Svg::new(output.stdout))
            } else {
                Err(SubplotError::child_failed("plantuml", &output))
            }
        } else {
            Err(SubplotError::ChildNoStdin)
        }
    }
}
