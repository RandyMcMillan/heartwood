use crate::html::Location;
use crate::{resource, Document, SubplotError, TemplateSpec};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use base64::prelude::{Engine as _, BASE64_STANDARD};

use serde::Serialize;
use tera::{Context, Tera, Value};

/// Generate a test program from a document, using a template spec.
pub fn generate_test_program(
    doc: &mut Document,
    filename: &Path,
    template: &str,
) -> Result<(), SubplotError> {
    let context = context(doc, template)?;
    let docimpl = doc
        .meta()
        .document_impl(template)
        .ok_or(SubplotError::MissingTemplate)?;

    let code = tera(docimpl.spec(), template)?
        .render("template", &context)
        .expect("render");
    write(filename, &code)?;
    Ok(())
}

fn context(doc: &mut Document, template: &str) -> Result<Context, SubplotError> {
    let mut context = Context::new();
    let scenarios = doc.matched_scenarios(template)?;
    context.insert("scenarios", &scenarios);
    context.insert("files", doc.embedded_files());

    let mut funcs = vec![];
    if let Some(docimpl) = doc.meta().document_impl(template) {
        for filename in docimpl.functions_filenames() {
            let content = resource::read_as_string(filename, Some(template))
                .map_err(|err| SubplotError::FunctionsFileNotFound(filename.into(), err))?;
            funcs.push(Func::new(filename, content));
        }
    }
    context.insert("functions", &funcs);

    // Any of the above could fail for more serious reasons, but if we get this far
    // and our context would have no scenarios in it, then we complain.
    if scenarios.is_empty() {
        return Err(SubplotError::NoScenariosMatched(template.to_string()));
    }
    Ok(context)
}

fn tera(tmplspec: &TemplateSpec, templatename: &str) -> Result<Tera, SubplotError> {
    let mut tera = Tera::default();
    tera.register_filter("base64", base64);
    tera.register_filter("nameslug", nameslug);
    tera.register_filter("commentsafe", commentsafe);
    tera.register_filter("location", locationfilter);
    let dirname = tmplspec.template_filename().parent().unwrap();
    for helper in tmplspec.helpers() {
        let helper_path = dirname.join(helper);
        let helper_content = resource::read_as_string(&helper_path, Some(templatename))
            .map_err(|err| SubplotError::ReadFile(helper_path.clone(), err))?;
        let helper_name = helper.display().to_string();
        tera.add_raw_template(&helper_name, &helper_content)
            .map_err(|err| SubplotError::TemplateError(helper_name.to_string(), err))?;
    }
    let path = tmplspec.template_filename();
    let template = resource::read_as_string(path, Some(templatename))
        .map_err(|err| SubplotError::ReadFile(path.to_path_buf(), err))?;
    tera.add_raw_template("template", &template)
        .map_err(|err| {
            SubplotError::TemplateError(tmplspec.template_filename().display().to_string(), err)
        })?;
    Ok(tera)
}

fn write(filename: &Path, content: &str) -> Result<(), SubplotError> {
    let mut f: File = File::create(filename)
        .map_err(|err| SubplotError::CreateFile(filename.to_path_buf(), err))?;
    f.write_all(content.as_bytes())
        .map_err(|err| SubplotError::WriteFile(filename.to_path_buf(), err))?;
    Ok(())
}

fn base64(v: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
    match v {
        Value::String(s) => Ok(Value::String(BASE64_STANDARD.encode(s))),
        _ => Err(tera::Error::msg(
            "can only base64 encode strings".to_string(),
        )),
    }
}

fn locationfilter(v: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
    let location: Location = serde_json::from_value(v.clone())?;
    Ok(Value::String(format!(
        "{:?}",
        match location {
            Location::Known {
                filename,
                line,
                col,
            } => format!("{}:{}:{}", filename.display(), line, col),
            Location::Unknown => "unknown".to_string(),
        }
    )))
}

fn nameslug(name: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
    match name {
        Value::String(s) => {
            let newname = s
                .chars()
                .map(|c| match c {
                    'a'..='z' => c,
                    'A'..='Z' => c.to_ascii_lowercase(),
                    _ => '_',
                })
                .collect();
            Ok(Value::String(newname))
        }
        _ => Err(tera::Error::msg(
            "can only create nameslugs from strings".to_string(),
        )),
    }
}

fn commentsafe(s: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
    match s {
        Value::String(s) => {
            let cleaned = s
                .chars()
                .map(|c| match c {
                    '\n' | '\r' => ' ',
                    _ => c,
                })
                .collect();
            Ok(Value::String(cleaned))
        }
        _ => Err(tera::Error::msg(
            "can only make clean comments from strings".to_string(),
        )),
    }
}

#[derive(Debug, Serialize)]
pub struct Func {
    pub source: PathBuf,
    pub code: String,
}

impl Func {
    pub fn new(source: &Path, code: String) -> Func {
        Func {
            source: source.to_path_buf(),
            code,
        }
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;
    use tera::Value;
    #[test]
    fn verify_commentsafe_filter() {
        static GOOD_CASES: &[(&str, &str)] = &[
            ("", ""),                                         // Empty
            ("hello world", "hello world"),                   // basic strings pass through
            ("Capitalised Words", "Capitalised Words"),       // capitals are OK
            ("multiple\nlines\rblah", "multiple lines blah"), // line breaks are made into spaces
        ];
        for (input, output) in GOOD_CASES.iter().copied() {
            let input = Value::from(input);
            let output = Value::from(output);
            let empty = HashMap::new();
            assert_eq!(super::commentsafe(&input, &empty).ok(), Some(output));
        }
    }

    #[test]
    fn verify_name_slugification() {
        static GOOD_CASES: &[(&str, &str)] = &[
            ("foobar", "foobar"),       // Simple words pass through
            ("FooBar", "foobar"),       // Capital letters are lowercased
            ("Motörhead", "mot_rhead"), // Non-ascii characters are changed for underscores
            ("foo bar", "foo_bar"),     // As is whitespace etc.
        ];
        for (input, output) in GOOD_CASES.iter().copied() {
            let input = Value::from(input);
            let output = Value::from(output);
            let empty = HashMap::new();
            assert_eq!(super::nameslug(&input, &empty).ok(), Some(output));
        }
    }
}
