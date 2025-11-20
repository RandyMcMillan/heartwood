use std::path::Path;
use subplotlib::steplibrary::files::{self, Datadir};
use subplotlib::steplibrary::runcmd::Runcmd;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[step]
#[context(Datadir)]
fn create_script_from_embedded(
    context: &ScenarioContext,
    filename: &Path,
    embedded: SubplotDataFile,
) {
    files::create_from_embedded_with_other_name::call(context, filename, embedded)?;
    let filename = context.with(|dd: &Datadir| dd.canonicalise_filename(filename), false)?;
    let mut perms = std::fs::symlink_metadata(&filename)?.permissions();
    #[cfg(unix)]
    perms.set_mode(perms.mode() | 0o111);
    std::fs::set_permissions(&filename, perms)?;
}

#[step]
fn prepend_to_path(context: &mut Runcmd, dirname: &Path) {
    context.prepend_to_path(dirname);
}
