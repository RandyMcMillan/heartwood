use std::env;
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let hash = env::var("GIT_HEAD").unwrap_or_else(|_| {
        Command::new("git")
            .arg("rev-parse")
            .arg("--short")
            .arg("HEAD")
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    String::from_utf8(output.stdout).ok()
                } else {
                    None
                }
            })
            .unwrap_or("unknown".into())
    });

    let version = env::var("RADICLE_VERSION").unwrap_or_else(|_| {
        Command::new("git")
            .arg("describe")
            .arg("--always")
            .arg("--broken")
            .arg("--dirty")
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    String::from_utf8(output.stdout).ok()
                } else {
                    None
                }
            })
            .unwrap_or(hash.clone())
    });

    let version = if let Some(stripped) = version.strip_prefix("releases/") {
        stripped.to_owned()
    } else {
        version
    };

    let commit_time = env::var("SOURCE_DATE_EPOCH").unwrap_or_else(|_| {
        Command::new("git")
            .arg("log")
            .arg("-1")
            .arg("--pretty=%ct")
            .arg("HEAD")
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    String::from_utf8(output.stdout).ok()
                } else {
                    None
                }
            })
            .unwrap_or(0.to_string())
    });

    println!("cargo::rustc-env=RADICLE_VERSION={version}");
    println!("cargo::rustc-env=SOURCE_DATE_EPOCH={commit_time}");
    println!("cargo::rustc-env=GIT_HEAD={hash}");

    Ok(())
}
