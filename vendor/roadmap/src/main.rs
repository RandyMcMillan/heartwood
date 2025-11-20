use std::{
    fs::{read, write},
    path::PathBuf,
    process::Command,
};

use clap::Parser;
use tempfile::tempdir;

use roadmap::from_yaml;

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let data = read(&args.filename)?;
    let yaml = String::from_utf8(data)?;
    let roadmap = from_yaml(&yaml)?;
    let graph = roadmap.format_as_dot(args.label_width)?;

    let tmp = tempdir()?;
    let filename = tmp.path().join("dot");
    write(&filename, graph)?;

    let output = Command::new("dot").arg("-Tsvg").arg(filename).output()?;
    if output.status.success() {
        let dot = String::from_utf8(output.stdout)?;
        println!("{}", dot);
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("ERROR: dot failed:\n{stderr}");
        std::process::exit(output.status.code().unwrap());
    }

    Ok(())
}

#[derive(Parser)]
struct Args {
    /// Width of labels in the graph in characters.
    #[clap(short, long, default_value = "20")]
    label_width: usize,

    /// Input filename.
    filename: PathBuf,
}
