use crate::resource;
use crate::SubplotError;

use serde::Deserialize;

use std::path::{Path, PathBuf};

/// A template specification.
///
/// Contains the information codegen needs to use a template for
/// generating a test program. The names and types of fields match the
/// files in the `templates/.../template.yaml` files.
#[derive(Debug, Deserialize)]
pub struct TemplateSpec {
    template: PathBuf,
    #[serde(default)]
    helpers: Vec<PathBuf>,
    run: Option<String>,
}

impl TemplateSpec {
    // Create a new TemplateSpec from YAML text.
    fn from_yaml(yaml: &str) -> Result<TemplateSpec, SubplotError> {
        marked_yaml::from_yaml(0 /* TODO: Keep track of sources */, yaml)
            .map_err(SubplotError::Metadata)
    }

    // Create a new TemplateSpec.
    fn new(
        basedir: &Path,
        template: &Path,
        helpers: Vec<PathBuf>,
        run: Option<&str>,
    ) -> TemplateSpec {
        TemplateSpec {
            template: basedir.to_path_buf().join(template),
            helpers,
            run: run.map(str::to_string),
        }
    }

    /// Read a template.yaml file and create the corresponding TemplateSpec.
    pub fn from_file(filename: &Path) -> Result<TemplateSpec, SubplotError> {
        let yaml = resource::read_as_string(filename, None)
            .map_err(|err| SubplotError::ReadFile(filename.to_path_buf(), err))?;
        let spec = TemplateSpec::from_yaml(&yaml)?;
        let dirname = match filename.parent() {
            Some(x) => x,
            None => {
                return Err(SubplotError::NoTemplateSpecDirectory(
                    filename.to_path_buf(),
                ))
            }
        };
        Ok(TemplateSpec::new(
            dirname,
            spec.template_filename(),
            spec.helpers().map(|p| p.to_path_buf()).collect(),
            spec.run(),
        ))
    }

    /// Return the name of the template file.
    pub fn template_filename(&self) -> &Path {
        &self.template
    }

    /// Return iterator for names of helper files.
    pub fn helpers(&self) -> impl Iterator<Item = &Path> {
        self.helpers.iter().map(|p| p.as_path())
    }

    /// Return command to run the generated test program, if specified.
    ///
    /// The name of the test program gets appended.
    pub fn run(&self) -> Option<&str> {
        self.run.as_deref()
    }
}

#[cfg(test)]
mod test {
    use super::TemplateSpec;

    #[test]
    fn new_from_yaml() {
        let yaml = "
template: template.py
";
        let spec = TemplateSpec::from_yaml(yaml).unwrap();
        assert_eq!(spec.template_filename().to_str().unwrap(), "template.py");
    }
}
