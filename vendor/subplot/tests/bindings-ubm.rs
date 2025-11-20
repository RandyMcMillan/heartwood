// A micro-benchmark for the Bindings struct.
//
// The goal is to see how it deals with looking up a binding when
// there are a large number of them.

use regex::RegexBuilder;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use std::{collections::HashMap, sync::Arc};
use subplot::{html::Location, Binding, Bindings, ScenarioStep, StepKind};

const N: i32 = 1000;

fn path() -> Arc<Path> {
    PathBuf::new().into()
}

#[test]
fn bindings_microbenchmark() {
    let time = SystemTime::now();

    let mut texts = vec![];
    for i in 0..N {
        texts.push(format!("step {i}"));
    }
    let texted = time.elapsed().unwrap();

    let mut re = vec![];
    for t in texts.iter() {
        re.push((
            t,
            RegexBuilder::new(&format!("^{t}$"))
                .case_insensitive(false)
                .build()
                .unwrap(),
        ));
    }
    let regexed = time.elapsed().unwrap();

    let mut toadd = vec![];
    for t in texts.iter() {
        toadd.push(Binding::new(StepKind::Given, t, false, HashMap::new(), None, path()).unwrap());
    }
    let created = time.elapsed().unwrap();

    let mut bindings = Bindings::new();
    for binding in toadd {
        bindings.add(binding);
    }
    let added = time.elapsed().unwrap();
    let step = ScenarioStep::new(
        StepKind::Given,
        "given",
        &format!("step {}", N - 1),
        Location::Unknown,
    );
    bindings.find("", &step).unwrap();
    let found = time.elapsed().unwrap();

    println!("texted: {}", texted.as_millis());
    println!("regexed: {}", regexed.as_millis());
    println!("created: {}", created.as_millis());
    println!("added: {}", added.as_millis());
    println!("found: {}", found.as_millis());
}
