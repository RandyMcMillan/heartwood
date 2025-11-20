// Build script for subplotlib.
//
// We use the `subplot_build` crate to generate a Rust test code file
// with functions for the scenarios for each subplot file
// (subplot/*.subplot in the source tree). The generated file is written to
// the Cargo target directory. For each subplot foo.subplot there should be
// a tests/foo.rs in the source tree, which includes the generated
// file from the Cargo target tree. The source file should look like this:
//
// ```rust
// include!(concat!(env!("OUT_DIR"), "/foo.rs"));
// ```

use glob::glob;
use std::{fs, path::Path};

fn gen_tests() -> Result<(), subplot_build::SubplotError> {
    let subplots = glob("*.subplot").expect("failed to find subplots in subplotlib");
    let tests = Path::new("tests");
    let subplots = subplots.chain(Some(Ok("../subplot.subplot".into())));
    let subplots = subplots
        .chain(glob("../tests/subplots/common/*.subplot").expect("failed to find common subplots"));
    for entry in subplots {
        let entry = entry.expect("failed to get subplot dir entry in subplotlib");
        let mut inc = tests.join(entry.file_name().unwrap());
        inc.set_extension("rs");
        if !inc.exists() {
            panic!("missing include file: {}", inc.display());
        }
        println!("cargo:rerun-if-changed={}", inc.display());
        println!("cargo:rerun-if-changed={}", entry.display());
        subplot_build::codegen(Path::new(&entry)).expect("failed to generate code with Subplot");
    }
    Ok(())
}

fn main() {
    // Because we cannot generate tests if we're not fully inside the main subplot tree
    // we only generate them if we can see ../subplot.subplot which is a good indicator.
    if fs::metadata("../subplot.subplot").is_ok() {
        if let Err(e) = gen_tests() {
            eprintln!("Failed to generate code from subplot: {e}");
            std::process::exit(1);
        }
    }
}
