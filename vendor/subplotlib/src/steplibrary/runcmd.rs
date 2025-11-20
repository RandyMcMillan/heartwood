//! Step library for running subprocesses as part of scenarios

use regex::RegexBuilder;

pub use super::datadir::Datadir;
pub use crate::prelude::*;

use std::collections::HashMap;
use std::env::{self, JoinPathsError};
use std::ffi::{OsStr, OsString};
use std::fmt::Debug;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The Runcmd context gives a step function access to the ability to run
/// subprocesses as part of a scenario.  These subprocesses are run with
/// various environment variables set, and we record the stdout/stderr
/// of the most recent-to-run command for testing purposes.
#[derive(Default)]
pub struct Runcmd {
    env: HashMap<OsString, OsString>,
    // push to "prepend", order reversed when added to env
    paths: Vec<OsString>,
    // The following are the result of any executed command
    exitcode: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl Debug for Runcmd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runcmd")
            .field("env", &self.env)
            .field("paths", &self.paths)
            .field("exitcode", &self.exitcode)
            .field("stdout", &String::from_utf8_lossy(&self.stdout))
            .field("stderr", &String::from_utf8_lossy(&self.stderr))
            .finish()
    }
}

// Note, this prefix requires that the injection env vars must have
// names which are valid unicode (and ideally ASCII)
const ENV_INJECTION_PREFIX: &str = "SUBPLOT_ENV_";

#[cfg(not(windows))]
static DEFAULT_PATHS: &[&str] = &["/usr/bin", "/bin"];

// Note, this comes from https://www.computerhope.com/issues/ch000549.htm#defaultpath
#[cfg(windows)]
static DEFAULT_PATHS: &[&str] = &[
    r"%SystemRoot%\system32",
    r"%SystemRoot%",
    r"%SystemRoot%\System32\Wbem",
];

// This us used internally to force CWD for running commands
lazy_static! {
    static ref USE_CWD: PathBuf = PathBuf::from("\0USE_CWD");
}

impl ContextElement for Runcmd {
    fn scenario_starts(&mut self) -> StepResult {
        self.env.drain();
        self.paths.drain(..);
        self.env.insert("SHELL".into(), "/bin/sh".into());
        self.env.insert(
            "PATH".into(),
            env::var_os("PATH")
                .map(Ok)
                .unwrap_or_else(|| env::join_paths(DEFAULT_PATHS.iter()))?,
        );

        // Having assembled the 'default' environment, override it with injected
        // content from the calling environment.
        for (k, v) in env::vars_os() {
            if let Some(k) = k.to_str() {
                if let Some(k) = k.strip_prefix(ENV_INJECTION_PREFIX) {
                    self.env.insert(k.into(), v);
                }
            }
        }
        Ok(())
    }
}

impl Runcmd {
    /// Prepend the given location to the run path
    pub fn prepend_to_path<S: Into<OsString>>(&mut self, element: S) {
        self.paths.push(element.into());
    }

    /// Retrieve the last run command's stdout as a string.
    ///
    /// This does a lossy conversion from utf8 so should always succeed.
    pub fn stdout_as_string(&self) -> String {
        String::from_utf8_lossy(&self.stdout).into_owned()
    }

    /// Retrieve the last run command's stderr as a string.
    ///
    /// This does a lossy conversion from utf8 so should always succeed.
    pub fn stderr_as_string(&self) -> String {
        String::from_utf8_lossy(&self.stderr).into_owned()
    }

    /// Set an env var in the Runcmd context
    ///
    /// This sets an environment variable into the Runcmd context for use
    /// during execution
    pub fn setenv<K: Into<OsString>, V: Into<OsString>>(&mut self, key: K, value: V) {
        self.env.insert(key.into(), value.into());
    }

    /// Get an env var from the Runcmd context
    ///
    /// This retrieves a set environment variable from the Runcmd context
    pub fn getenv<K: AsRef<OsStr>>(&self, key: K) -> Option<&OsStr> {
        self.env.get(key.as_ref()).map(OsString::as_os_str)
    }

    /// Unset an env var in the Runcmd context
    ///
    /// This removes an environment variable (if set) from the Runcmd context
    /// and returns whether or not it was removed.
    pub fn unsetenv<K: AsRef<OsStr>>(&mut self, key: K) -> bool {
        self.env.remove(key.as_ref()).is_some()
    }

    /// Join the `PATH` environment variable and the `paths` attribute
    /// together properly.
    ///
    /// This prepends the paths (in reverse order) to the `PATH` environment
    /// variable and then returns it.
    ///
    /// If there is no `PATH` in the stored environment then the resultant
    /// path will be entirely made up of the pushed path elements
    ///
    /// ```
    /// # use subplotlib::steplibrary::runcmd::Runcmd;
    ///
    /// let mut rc = Runcmd::default();
    ///
    /// assert_eq!(rc.join_paths().unwrap(), "");
    ///
    /// rc.setenv("PATH", "one");
    /// assert_eq!(rc.join_paths().unwrap(), "one");
    ///
    /// rc.prepend_to_path("two");
    /// assert_eq!(rc.join_paths().unwrap(), "two:one");
    ///
    /// rc.unsetenv("PATH");
    /// assert_eq!(rc.join_paths().unwrap(), "two");
    ///
    /// rc.prepend_to_path("three");
    /// assert_eq!(rc.join_paths().unwrap(), "three:two");
    ///
    /// rc.setenv("PATH", "one");
    /// assert_eq!(rc.join_paths().unwrap(), "three:two:one");
    /// ```
    ///
    pub fn join_paths(&self) -> Result<OsString, JoinPathsError> {
        let curpath = self
            .env
            .get(OsStr::new("PATH"))
            .map(|s| s.as_os_str())
            .unwrap_or_else(|| OsStr::new(""));
        env::join_paths(
            self.paths
                .iter()
                .rev()
                .map(PathBuf::from)
                .chain(env::split_paths(curpath).filter(|p| p != Path::new(""))),
        )
    }
}

/// Ensure the given data file is available as a script in the data dir
///
/// # `given helper script {script} for runcmd`
///
/// ## Note
///
/// Currently this does not make the script file executable, so you will
/// need to invoke it by means of an interpreter
#[step]
pub fn helper_script(context: &Datadir, script: SubplotDataFile) {
    context
        .open_write(script.name())?
        .write_all(script.data())?;
}

/// Ensure that the base source directory is in the `PATH` for subsequent
/// commands being run.
///
/// # `given srcdir is in the PATH`
///
/// This inserts the `CARGO_MANIFEST_DIR` into the `PATH` environment
/// variable at the front.
#[step]
pub fn helper_srcdir_path(context: &mut Runcmd) {
    context.prepend_to_path(env!("CARGO_MANIFEST_DIR"));
}

/// Run the given command, ensuring it succeeds
///
/// # `when I run {argv0}{args:text}`
///
/// This will run the given command, with the given arguments,
/// in the "current" directory (from where the tests were invoked)
/// Once the command completes, this will check that it exited with
/// a zero code (success).
#[step]
#[context(Datadir)]
#[context(Runcmd)]
pub fn run(context: &ScenarioContext, argv0: &str, args: &str) {
    try_to_run::call(context, argv0, args)?;
    exit_code_is::call(context, 0)?;
}

/// Run the given command in the given subpath of the data directory,
/// ensuring that it succeeds.
///
/// # `when I run, in {dirname}, {argv0}{args:text}`
///
/// Like `run`, this will execute the given command, but this time it
/// will set the working directory to the given subpath of the data dir.
/// Once the command completes, this will check that it exited with
/// a zero code (success)
#[step]
#[context(Datadir)]
#[context(Runcmd)]
pub fn run_in(context: &ScenarioContext, dirname: &Path, argv0: &str, args: &str) {
    try_to_run_in::call(context, dirname, argv0, args)?;
    exit_code_is::call(context, 0)?;
}

/// Run the given command
///
/// # `when I try to run {argv0}{args:text}`
///
/// This will run the given command, with the given arguments,
/// in the "current" directory (from where the tests were invoked)
#[step]
#[context(Datadir)]
#[context(Runcmd)]
pub fn try_to_run(context: &ScenarioContext, argv0: &str, args: &str) {
    try_to_run_in::call(context, &USE_CWD, argv0, args)?;
}

/// Run the given command in the given subpath of the data directory
///
/// # `when I try to run, in {dirname}, {argv0}{args:text}`
///
/// Like `try_to_run`, this will execute the given command, but this time it
/// will set the working directory to the given subpath of the data dir.
#[step]
#[context(Datadir)]
#[context(Runcmd)]
pub fn try_to_run_in(context: &ScenarioContext, dirname: &Path, argv0: &str, args: &str) {
    // This is the core of runcmd and is how we handle things
    let argv0: PathBuf = if argv0.starts_with('.') {
        context.with(
            |datadir: &Datadir| datadir.canonicalise_filename(argv0),
            false,
        )?
    } else {
        argv0.into()
    };
    let mut datadir = context.with(
        |datadir: &Datadir| Ok(datadir.base_path().to_path_buf()),
        false,
    )?;
    if dirname != USE_CWD.as_path() {
        datadir = datadir.join(dirname);
    }
    let mut proc = Command::new(&argv0);
    let args = shell_words::split(args)?;
    proc.args(&args);
    proc.current_dir(&datadir);

    println!(
        "Running `{}` with args {:?}\nRunning in {}",
        argv0.display(),
        args,
        datadir.display()
    );
    proc.env("HOME", &datadir);
    proc.env("TMPDIR", &datadir);

    context.with(
        |runcmd: &Runcmd| {
            for (k, v) in runcmd
                .env
                .iter()
                .filter(|(k, _)| k.to_str() != Some("PATH"))
            {
                println!("ENV: {} = {}", k.to_string_lossy(), v.to_string_lossy());
                proc.env(k, v);
            }
            Ok(())
        },
        false,
    )?;

    let path = context.with(|runcmd: &Runcmd| Ok(runcmd.join_paths()?), false)?;
    proc.env("PATH", &path);
    println!("PATH: {}", path.to_string_lossy());
    proc.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut output = proc.output()?;
    context.with_mut(
        |runcmd: &mut Runcmd| {
            std::mem::swap(&mut runcmd.stdout, &mut output.stdout);
            std::mem::swap(&mut runcmd.stderr, &mut output.stderr);
            runcmd.exitcode = output.status.code();
            println!("Exit code: {}", runcmd.exitcode.unwrap_or(-1));
            println!(
                "Stdout:\n{}\nStderr:\n{}\n",
                runcmd.stdout_as_string(),
                runcmd.stderr_as_string()
            );
            Ok(())
        },
        false,
    )?;
}

/// Check that an executed command returns a specific exit code
///
/// # `then exit code is {exit}`
///
/// Check that the exit code of the previously run command matches
/// the given value.  Typically zero is success.
#[step]
pub fn exit_code_is(context: &Runcmd, exit: i32) {
    if context.exitcode != Some(exit) {
        throw!(format!(
            "expected exit code {}, but had {:?}",
            exit, context.exitcode
        ));
    }
}

/// Check that an executed command returns a specific exit code
///
/// # `then exit code is not {exit}`
///
/// Check that the exit code of the previously run command does not
/// matche the given value.
#[step]
pub fn exit_code_is_not(context: &Runcmd, exit: i32) {
    if context.exitcode.is_none() || context.exitcode == Some(exit) {
        throw!(format!("Expected exit code to not equal {exit}"));
    }
}

/// Check that an executed command succeeded
///
/// # `then command is successful`
///
/// This is equivalent to `then exit code is 0`
#[step]
#[context(Runcmd)]
pub fn exit_code_is_zero(context: &ScenarioContext) {
    exit_code_is::call(context, 0)?;
}

/// Check that an executed command did not succeed
///
/// # `then command fails`
///
/// This is equivalent to `then exit code is not 0`
#[step]
#[context(Runcmd)]
pub fn exit_code_is_nonzero(context: &ScenarioContext) {
    exit_code_is_not::call(context, 0)?;
}

enum Stream {
    Stdout,
    Stderr,
}
enum MatchKind {
    Exact,
    Contains,
    Regex,
}

#[throws(StepError)]
fn check_matches(runcmd: &Runcmd, which: Stream, how: MatchKind, against: &str) -> bool {
    let stream = match which {
        Stream::Stdout => &runcmd.stdout,
        Stream::Stderr => &runcmd.stderr,
    };
    let against = if matches!(how, MatchKind::Regex) {
        against.to_string()
    } else {
        unescape::unescape(against).ok_or("unable to unescape input")?
    };
    match how {
        MatchKind::Exact => stream.as_slice() == against.as_bytes(),
        MatchKind::Contains => stream
            .windows(against.len())
            .any(|window| window == against.as_bytes()),
        MatchKind::Regex => {
            let stream = String::from_utf8_lossy(stream);
            let regex = RegexBuilder::new(&against).multi_line(true).build()?;
            regex.is_match(&stream)
        }
    }
}

/// Check that the stdout of the command matches exactly
///
/// # `then stdout is exactly "{text:text}"`
///
/// This will check exactly that the stdout of the command matches the given
/// text.  This assumes the command outputs valid utf-8 and decodes it as such.
#[step]
pub fn stdout_is(runcmd: &Runcmd, text: &str) {
    if !check_matches(runcmd, Stream::Stdout, MatchKind::Exact, text)? {
        throw!(format!("stdout is not {text:?}"));
    }
}

/// Check that the stdout of the command is exactly not a given value
///
/// # `then stdout isn't exactly "{text:text}"`
///
/// This will check exactly that the stdout of the command does not match the given
/// text.  This assumes the command outputs valid utf-8 and decodes it as such.
#[step]
pub fn stdout_isnt(runcmd: &Runcmd, text: &str) {
    if check_matches(runcmd, Stream::Stdout, MatchKind::Exact, text)? {
        throw!(format!("stdout is exactly {text:?}"));
    }
}

/// Check that the stderr of the command matches exactly
///
/// # `then stderr is exactly "{text:text}"`
///
/// This will check exactly that the stderr of the command matches the given
/// text.  This assumes the command outputs valid utf-8 and decodes it as such.
#[step]
pub fn stderr_is(runcmd: &Runcmd, text: &str) {
    if !check_matches(runcmd, Stream::Stderr, MatchKind::Exact, text)? {
        throw!(format!("stderr is not {text:?}"));
    }
}

/// Check that the stderr of the command is exactly not a given value
///
/// # `then stderr isn't exactly "{text:text}"`
///
/// This will check exactly that the stderr of the command does not match the given
/// text.  This assumes the command outputs valid utf-8 and decodes it as such.
#[step]
pub fn stderr_isnt(runcmd: &Runcmd, text: &str) {
    if check_matches(runcmd, Stream::Stderr, MatchKind::Exact, text)? {
        throw!(format!("stderr is exactly {text:?}"));
    }
}

/// Check that the stdout of the command contains a given string
///
/// # `then stdout contains "{text:text}"`
///
/// This will check that the stdout of the command contains the given substring. This
/// assumes the command outputs valid utf-8 and decodes it as such.
#[step]
pub fn stdout_contains(runcmd: &Runcmd, text: &str) {
    if !check_matches(runcmd, Stream::Stdout, MatchKind::Contains, text)? {
        throw!(format!("stdout does not contain {text:?}"));
    }
}

/// Check that the stdout of the command does not contain a given string
///
/// # `then stdout doesn't contain "{text:text}"`
///
/// This will check that the stdout of the command does not contain the given substring. This
/// assumes the command outputs valid utf-8 and decodes it as such.
#[step]
pub fn stdout_doesnt_contain(runcmd: &Runcmd, text: &str) {
    if check_matches(runcmd, Stream::Stdout, MatchKind::Contains, text)? {
        throw!(format!("stdout contains {text:?}"));
    }
}

/// Check that the stderr of the command contains a given string
///
/// # `then stderr contains "{text:text}"`
///
/// This will check that the stderr of the command contains the given substring. This
/// assumes the command outputs valid utf-8 and decodes it as such.
#[step]
pub fn stderr_contains(runcmd: &Runcmd, text: &str) {
    if !check_matches(runcmd, Stream::Stderr, MatchKind::Contains, text)? {
        throw!(format!("stderr does not contain {text:?}"));
    }
}

/// Check that the stderr of the command does not contain a given string
///
/// # `then stderr doesn't contain "{text:text}"`
///
/// This will check that the stderr of the command does not contain the given substring. This
/// assumes the command outputs valid utf-8 and decodes it as such.
#[step]
pub fn stderr_doesnt_contain(runcmd: &Runcmd, text: &str) {
    if check_matches(runcmd, Stream::Stderr, MatchKind::Contains, text)? {
        throw!(format!("stderr contains {text:?}"));
    }
}

/// Check that the stdout of the command matches a given regular expression
///
/// # `then stdout matches regex {regex:text}`
///
/// This will check that the stdout of the command matches the given regular expression.
/// This will fail if the regular expression is bad, or if the command did not output
/// valid utf-8 to be decoded.
#[step]
pub fn stdout_matches_regex(runcmd: &Runcmd, regex: &str) {
    if !check_matches(runcmd, Stream::Stdout, MatchKind::Regex, regex)? {
        throw!(format!("stdout does not match {regex:?}"));
    }
}

/// Check that the stdout of the command does not match a given regular expression
///
/// # `then stdout doesn't match regex {regex:text}`
///
/// This will check that the stdout of the command fails to match the given regular expression.
/// This will fail if the regular expression is bad, or if the command did not output
/// valid utf-8 to be decoded.
#[step]
pub fn stdout_doesnt_match_regex(runcmd: &Runcmd, regex: &str) {
    if check_matches(runcmd, Stream::Stdout, MatchKind::Regex, regex)? {
        throw!(format!("stdout matches {regex:?}"));
    }
}

/// Check that the stderr of the command matches a given regular expression
///
/// # `then stderr matches regex {regex:text}`
///
/// This will check that the stderr of the command matches the given regular expression.
/// This will fail if the regular expression is bad, or if the command did not output
/// valid utf-8 to be decoded.
#[step]
pub fn stderr_matches_regex(runcmd: &Runcmd, regex: &str) {
    if !check_matches(runcmd, Stream::Stderr, MatchKind::Regex, regex)? {
        throw!(format!("stderr does not match {regex:?}"));
    }
}

/// Check that the stderr of the command does not match a given regular expression
///
/// # `then stderr doesn't match regex {regex:text}`
///
/// This will check that the stderr of the command fails to match the given regular expression.
/// This will fail if the regular expression is bad, or if the command did not output
/// valid utf-8 to be decoded.
#[step]
pub fn stderr_doesnt_match_regex(runcmd: &Runcmd, regex: &str) {
    if check_matches(runcmd, Stream::Stderr, MatchKind::Regex, regex)? {
        throw!(format!("stderr matches {regex:?}"));
    }
}
