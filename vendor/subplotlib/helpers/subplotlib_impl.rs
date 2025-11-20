#[step]
fn a_trivial_setup(context: &mut Context, initial: usize) {
    context.counter = initial;
}

#[step]
fn a_trivial_cleanup(_context: &mut Context, _initial: usize) {}

#[step]
fn increment_counter(context: &mut Context) {
    context.counter += 1;
}

#[step]
fn internal_check_counter(context: &Context, num: usize) {
    if context.counter != num {
        throw!(format!(
            "Counter was wrong, it was {} but {} was expected",
            context.counter, num
        ));
    }
}

#[step]
fn check_counter(context: &ScenarioContext, num: usize) {
    internal_check_counter::call(context, num)?;
}

#[step]
fn acquire_file_content(context: &mut Context, somename: &str, file: SubplotDataFile) {
    context.remember_file(somename, file);
}

#[step]
fn remember_target(context: &mut Context, somename: &str) {
    if let Some(file) = context.files.get(somename) {
        context.this_file = Some(file.clone());
    } else {
        throw!(format!("Unknown file {somename}"));
    }
}

#[step]
fn check_contents(context: &mut Context, text: &str) {
    if let Some(file) = context.this_file.as_ref() {
        let body_as_text = String::from_utf8_lossy(file.data());
        if !body_as_text.as_ref().contains(text) {
            throw!(format!(
                "Failed to find {} when looking at {}",
                text,
                file.name().display()
            ));
        }
    } else {
        throw!("Not looking at a file");
    }
}
