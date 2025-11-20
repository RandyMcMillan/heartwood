//! Subplot test program building from `build.rs`
//!
//! This crate provides the Subplot code generation facility in a way
//! that's meant to be easy to use from another project's `build.rs`
//! module.

use std::env::var_os;
use std::error::Error;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use subplot::get_basedir_from;
pub use subplot::SubplotError;
use tracing::{event, instrument, span, Level};

/// Generate code for one document, inside `build.rs`.
///
/// The output files will be written to the directory specified in the
/// OUT_DIR environment variable. That variable is set by Cargo, when
/// a crate is built.
///
/// Also emit instructions for Cargo so it knows to re-run `build.rs`
/// whenever the input subplot or any of the bindings or functions
/// files it refers to changes. See
/// <https://doc.rust-lang.org/cargo/reference/build-scripts.html> for
/// details.
///
/// ```
/// use subplot_build::codegen;
/// # let dir = tempfile::tempdir().unwrap();
/// # std::env::set_var("OUT_DIR", dir.path());
///
/// codegen("foo.md").ok(); // ignoring error to keep example short
/// # dir.close().unwrap()
/// ```
#[instrument(level = "trace")]
pub fn codegen<P>(filename: P) -> Result<(), SubplotError>
where
    P: AsRef<Path> + Debug,
{
    let filename = filename.as_ref();
    match _codegen(filename) {
        Ok(()) => Ok(()),
        Err(e) => {
            eprintln!(
                "\n\n\nsubplot_build::codegen({}) failed: {e}",
                filename.display(),
            );
            let mut es = e.source();
            while let Some(source) = es {
                eprintln!("caused by: {source}");
                es = source.source();
            }
            eprintln!("\n\n");
            Err(e)
        }
    }
}

fn _codegen(filename: &Path) -> Result<(), SubplotError> {
    let span = span!(Level::TRACE, "codegen_buildrs");
    let _enter = span.enter();

    event!(Level::TRACE, "Generating code in build.rs");

    // Decide the name of the generated test program.
    let out_dir = var_os("OUT_DIR").expect("OUT_DIR is not defined in the environment");
    let out_dir = Path::new(&out_dir);
    let test_rs =
        buildrs_output(out_dir, filename, "rs").expect("could not create output filename");

    // Generate test program.
    let output = subplot::codegen(filename, &test_rs, Some("rust"))?;

    // Write instructions for Cargo to check if build scripts needs
    // re-running.
    let base_path = get_basedir_from(filename);
    let meta = output.doc.meta();
    for filename in meta.markdown_filenames() {
        buildrs_deps(&base_path, Some(filename.as_path()));
    }
    buildrs_deps(&base_path, meta.bindings_filenames());
    let docimpl = output
        .doc
        .meta()
        .document_impl("rust")
        .expect("We managed to codegen rust, yet the spec is missing?");
    buildrs_deps(&base_path, docimpl.functions_filenames());
    buildrs_deps(&base_path, Some(filename));

    event!(Level::TRACE, "Finished generating code");
    Ok(())
}

fn buildrs_deps<'a>(base_path: &Path, filenames: impl IntoIterator<Item = &'a Path>) {
    for filename in filenames {
        let filename = base_path.join(filename);
        if filename.exists() {
            println!("cargo:rerun-if-changed={}", filename.display());
        }
    }
}

fn buildrs_output(dir: &Path, filename: &Path, new_extension: &str) -> Option<PathBuf> {
    if let Some(basename) = filename.file_name() {
        let basename = Path::new(basename);
        if let Some(stem) = basename.file_stem() {
            let stem = Path::new(stem);
            return Some(dir.join(stem.with_extension(new_extension)));
        }
    }
    None
}
