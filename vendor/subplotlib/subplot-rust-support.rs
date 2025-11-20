// Rust support for running subplot-rust.md

use subplotlib::steplibrary::datadir::Datadir;
use subplotlib::steplibrary::runcmd::{self, Runcmd};

use tempfile::TempDir;

use std::io::{Read, Seek, SeekFrom};

#[derive(Debug, Default)]
struct SubplotContext {
    bin_dir: Option<TempDir>,
}

impl ContextElement for SubplotContext {}

#[step]
fn do_nothing(_context: &ScenarioContext) {
    // Nothing to do here
}

#[step]
#[context(SubplotContext)]
#[context(Runcmd)]
#[allow(clippy::single_element_loop)]
fn install_subplot(context: &ScenarioContext) {
    if let Some(bindir) = std::env::var_os("SUBPLOT_DIR") {
        println!("Found SUBPLOT_DIR environment variable, using that");
        context.with_mut(
            |rc: &mut Runcmd| {
                rc.prepend_to_path(bindir);
                Ok(())
            },
            false,
        )?;
    } else {
        let bin_dir = TempDir::new()?;
        println!("Creating temporary rundir at {}", bin_dir.path().display());

        // Since we don't get CARGO_BIN_EXE_subplot when building a subcrate
        // we retrieve the path to `subplot` via the assumption that integration
        // tests are always located one dir down from the outer crate binaries.
        let target_path = std::fs::canonicalize(
            std::env::current_exe()
                .expect("Cannot determine test exe path")
                .parent()
                .unwrap()
                .join(".."),
        )
        .expect("Cannot canonicalise path to binaries");

        let src_dir = env!("CARGO_MANIFEST_DIR");
        for bin_name in &["subplot"] {
            let file_path = bin_dir.path().join(bin_name);
            std::fs::write(
                &file_path,
                format!(
                    r#"
#!/bin/sh
set -eu
exec '{target_path}/{bin_name}' --resources '{src_dir}/share' "$@"
"#,
                    target_path = target_path.display(),
                ),
            )?;
            {
                let mut perms = std::fs::metadata(&file_path)?.permissions();
                use std::os::unix::fs::PermissionsExt;
                perms.set_mode(perms.mode() | 0o111); // Set executable bit
                std::fs::set_permissions(&file_path, perms)?;
            }
        }

        context.with_mut(
            |context: &mut Runcmd| {
                context.prepend_to_path(bin_dir.path());
                context.prepend_to_path(target_path);
                Ok(())
            },
            false,
        )?;
    }
}

#[step]
fn uninstall_subplot(context: &mut SubplotContext) {
    context.bin_dir.take();
}

#[step]
#[context(Runcmd)]
fn scenario_was_run(context: &ScenarioContext, name: &str) {
    let text = format!("\nscenario: {name}\n");
    runcmd::stdout_contains::call(context, &text)?;
}

#[step]
#[context(Runcmd)]
fn scenario_was_not_run(context: &ScenarioContext, name: &str) {
    let text = format!("\nscenario: {name}\n");
    runcmd::stdout_doesnt_contain::call(context, &text)?;
}

#[step]
#[context(Runcmd)]
fn step_was_run(context: &ScenarioContext, keyword: &str, name: &str) {
    let text = format!("\n  step: {keyword} {name}\n");
    runcmd::stdout_contains::call(context, &text)?;
}

#[step]
#[context(Runcmd)]
fn step_was_run_and_then(
    context: &ScenarioContext,
    keyword1: &str,
    name1: &str,
    keyword2: &str,
    name2: &str,
) {
    let text = format!("\n  step: {keyword1} {name1}\n  step: {keyword2} {name2}");
    runcmd::stdout_contains::call(context, &text)?;
}

#[step]
#[context(Runcmd)]
fn cleanup_was_run(
    context: &ScenarioContext,
    keyword1: &str,
    name1: &str,
    keyword2: &str,
    name2: &str,
) {
    let text = format!("\n  cleanup: {keyword1} {name1}\n  cleanup: {keyword2} {name2}\n");
    runcmd::stdout_contains::call(context, &text)?;
}

#[step]
#[context(Runcmd)]
fn cleanup_was_not_run(context: &ScenarioContext, keyword: &str, name: &str) {
    let text = format!("\n  cleanup: {keyword} {name}\n");
    runcmd::stdout_doesnt_contain::call(context, &text)?;
}

#[throws(StepError)]
fn end_of_file(context: &Datadir, filename: &str, nbytes: usize) -> Vec<u8> {
    let mut fh = context.open_read(filename)?;
    fh.seek(SeekFrom::End(-(nbytes as i64)))?;
    let mut b = vec![0; nbytes];
    fh.read_exact(&mut b[0..nbytes])?;
    b
}

#[step]
fn file_ends_in_zero_newlines(context: &Datadir, filename: &str) {
    let b = end_of_file(context, filename, 1)?;
    if b[0] == b'\n' {
        throw!(format!("File {filename} ends in unexpected newline"));
    }
}

#[step]
fn file_ends_in_one_newline(context: &Datadir, filename: &str) {
    let b = end_of_file(context, filename, 2)?;
    if !(b[0] != b'\n' && b[1] == b'\n') {
        throw!(format!(
            "File {filename} does not end in exactly one newline",
        ));
    }
}

#[step]
fn file_ends_in_two_newlines(context: &Datadir, filename: &str) {
    let b = end_of_file(context, filename, 2)?;
    if b[0] != b'\n' || b[1] != b'\n' {
        throw!(format!(
            "File {filename} does not end in exactly two newlines",
        ));
    }
}

#[step]
#[context(Datadir)]
#[context(Runcmd)]
fn json_output_matches_file(context: &ScenarioContext, filename: &str) {
    let output = context.with(|rc: &Runcmd| Ok(rc.stdout_as_string()), false)?;
    let fcontent = context.with(
        |dd: &Datadir| {
            Ok(std::fs::read_to_string(
                dd.canonicalise_filename(filename)?,
            )?)
        },
        false,
    )?;
    let output: serde_json::Value = serde_json::from_str(&output)?;
    let fcontent: serde_json::Value = serde_json::from_str(&fcontent)?;
    println!("########");
    println!("Output:\n{output:#}");
    println!("File:\n{fcontent:#}");
    println!("########");
    assert_eq!(
        output, fcontent,
        "Command output does not match the content of {filename}",
    );
}
