// Implementations of Subplot scenario steps for the CI broker.

use std::path::{Path, PathBuf};

use subplotlib::steplibrary::runcmd::Runcmd;

#[derive(Debug, Default)]
struct SubplotContext {}

impl ContextElement for SubplotContext {}

#[step]
#[context(SubplotContext)]
#[context(Runcmd)]
fn install_cib(context: &ScenarioContext) {
    let target_path = bindir();
    assert!(target_path.join("cib").exists());
    context.with_mut(
        |context: &mut Runcmd| {
            context.prepend_to_path(target_path);
            Ok(())
        },
        false,
    )?;
}

#[step]
#[context(SubplotContext)]
#[context(Runcmd)]
fn install_cibtool(context: &ScenarioContext) {
    let target_path = bindir();
    assert!(target_path.join("cibtool").exists());
    context.with_mut(
        |context: &mut Runcmd| {
            context.prepend_to_path(target_path);
            Ok(())
        },
        false,
    )?;
}

#[step]
#[context(SubplotContext)]
#[context(Runcmd)]
fn install_synthetic_events(context: &ScenarioContext) {
    let target_path = bindir();
    assert!(target_path.join("synthetic-events").exists());
    context.with_mut(
        |context: &mut Runcmd| {
            context.prepend_to_path(target_path);
            Ok(())
        },
        false,
    )?;
}

fn bindir() -> PathBuf {
    let path = if let Ok(target) = std::env::var("CARGO_TARGET_DIR") {
        Path::new(&target).join("debug")
    } else {
        PathBuf::from("target/debug")
    };
    path.canonicalize().unwrap()
}

#[step]
#[context(SubplotContext)]
#[context(Runcmd)]
fn stdout_has_one_line(runcmd: &Runcmd) {
    let linecount = runcmd.stdout_as_string().lines().count();
    if linecount != 1 {
        throw!(format!("stdout had {linecount} lines, expected 1"));
    }
}
