use std::collections::HashMap;

#[derive(Debug, Default)]
struct Context {
    counter: usize,
    files: HashMap<String, SubplotDataFile>,
    this_file: Option<SubplotDataFile>,
}

impl Context {
    fn remember_file(&mut self, name: &str, content: SubplotDataFile) {
        self.files.insert(name.to_string(), content);
    }
}

impl ContextElement for Context {
    // An empty implementation is sufficient for now
}
