use std::path::{Component, Path, PathBuf};

/// Get the base directory given the name of the markdown file.
///
/// All relative filename, such as bindings files, are resolved
/// against the base directory.
pub fn get_basedir_from(filename: &Path) -> PathBuf {
    match filename.parent() {
        None => {
            let p = Component::CurDir;
            let p: &Path = p.as_ref();
            p.to_path_buf()
        }
        Some(x) => x.to_path_buf(),
    }
}
