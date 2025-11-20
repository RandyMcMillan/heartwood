//! Read node events, expressed as JSON, from files, and output them as
//! single-line JSON objects to a Unix domain socket.
//!
//! This is a helper tool for testing of the CI broker as a whole.
//!
//! This program forks and runs itself in the background.

use std::{
    env::current_exe,
    fs::{read, remove_file, File, OpenOptions},
    io::{Read, Write},
    net::Shutdown,
    os::unix::net::{UnixListener, UnixStream},
    path::PathBuf,
    process::{Command, Stdio},
    thread::sleep,
    time::{Duration, Instant},
};

use clap::Parser;

use radicle::node::Event;

const MAX_WAIT: Duration = Duration::from_secs(10);

fn log(file: &mut File, msg: String) {
    file.write_all(msg.as_bytes()).ok();
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    if let Err(e) = {
        if args.client {
            client(&args)
        } else if args.daemon {
            daemon(&args)
        } else {
            launch_daemon(&args)
        }
    } {
        eprintln!("ERROR: {e}");
        let mut e = e.source();
        while let Some(underlying) = e {
            eprintln!("caused by: {underlying}");
            e = underlying.source();
        }
    }

    Ok(())
}

#[derive(Debug, Parser)]
struct Args {
    #[clap(long)]
    client: bool,
    #[clap(long)]
    daemon: bool,
    #[clap(long)]
    log: Option<PathBuf>,
    #[clap(long)]
    repeat: Option<usize>,
    socket: PathBuf,
    events: Vec<PathBuf>,
}

fn client(args: &Args) -> anyhow::Result<()> {
    println!("connect to {}", args.socket.display());
    {
        let mut stream = UnixStream::connect(&args.socket).unwrap();
        stream.shutdown(Shutdown::Write)?;
        let mut buf = vec![];
        stream.read_to_end(&mut buf)?;
    }

    println!("wait for daemon to remove the socket file");
    let started = Instant::now();
    while args.socket.exists() {
        let elapsed = started.elapsed();
        println!(
            "still waiting; it's been {} milliseconds",
            elapsed.as_millis()
        );
        if elapsed > MAX_WAIT {
            if let Some(filename) = args.log.as_ref() {
                let log = std::fs::read(filename)?;
                let log = String::from_utf8_lossy(&log);
                eprintln!("daemon log {}:\n====\n{log}\n----\n", filename.display());
            }
            panic!(
                "waited too long for {} to be deleted",
                args.socket.display()
            );
        }
        sleep(Duration::from_secs(1));
    }
    Ok(())
}

fn daemon(args: &Args) -> anyhow::Result<()> {
    fn helper(args: &Args, f: &mut File) -> anyhow::Result<()> {
        log(f, "daemon starts\n".into());

        let repeat = args.repeat.unwrap_or(1);
        log(f, format!("repeat={repeat}\n"));

        let mut events = vec![];
        for filename in args.events.iter() {
            log(f, format!("daemon reads file {}\n", filename.display()));

            let data = read(filename)?;
            let e: Event = serde_json::from_slice(&data)?;
            let mut e = serde_json::to_string(&e)?;
            e.push('\n');
            events.push(e);
        }

        log(f, format!("daemon connects to {}\n", args.socket.display()));
        let listener = UnixListener::bind(&args.socket)?;

        if let Some(stream) = listener.incoming().next() {
            log(f, format!("daemon accepted connection: {stream:?}\n"));
            let mut stream = stream?;
            log(f, format!("unwrapped stream: {stream:?}"));
            for e in events.iter() {
                log(f, "iterated to an event\n".into());
                for _ in 0..repeat {
                    stream.write_all(e.as_bytes())?;
                }
                log(f, "daemon sent a message\n".into());
            }
            log(f, "daemon sent all messages\n".into());

            log(f, "daemon shutting down write end\n".into());
            stream.shutdown(Shutdown::Write)?;
            log(f, "daemon shutdown write\n".into());

            // log(f, "daemon set socket read timeout\n".into());
            // stream.set_read_timeout(Some(Duration::from_millis(1000)))?;
            let mut buf = vec![];
            log(f, "daemon read from socket\n".into());
            stream.read_to_end(&mut buf).ok();
            log(f, "daemon finished reading\n".into());
        }

        log(f, "daemon removes socket file\n".into());
        remove_file(&args.socket)?;

        log(f, "daemon ends\n".into());

        Ok(())
    }

    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(args.log.as_ref().unwrap())?;

    if let Err(e) = helper(args, &mut f) {
        log(&mut f, format!("ERROR: {e}"));
    }

    Ok(())
}

fn launch_daemon(args: &Args) -> anyhow::Result<()> {
    if args.socket.exists() {
        println!("removing socket {}", args.socket.display());
        remove_file(&args.socket)?;
    }
    println!("launching daemon");
    let repeat = args.repeat.unwrap_or(1);
    let exe = current_exe()?;
    Command::new(exe)
        .arg("--daemon")
        .arg("--log")
        .arg(args.log.clone().unwrap_or(PathBuf::from("/dev/null")))
        .arg(&format!("--repeat={repeat}"))
        .arg(&args.socket)
        .args(&args.events)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    println!("waiting for daemon to create socket");
    while !args.socket.exists() {
        println!("no socket yet");
        sleep(Duration::from_millis(100));
    }
    println!("there is a socket now");

    Ok(())
}
