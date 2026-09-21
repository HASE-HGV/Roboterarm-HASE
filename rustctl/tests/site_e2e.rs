//! Subprocess tests against the real, compiled `rustctl` binary.
//!
//! These exist because `Some("--site") => run_site()` in `cli.rs`, and
//! `main()` itself, cannot be exercised by the in-process unit tests in
//! `src/tests.rs`: calling `run_site()` for real starts an accept loop that
//! never returns. Everything `run_site()` is built from - bind address
//! resolution, the startup banner, request parsing, routing, the page, the
//! busy gate, Origin checking - is unit-tested directly instead; this file
//! covers only the wiring that connects them to `--site` and `main()`.
//!
//! `CARGO_BIN_EXE_rustctl` is only set by Cargo for files under `tests/`,
//! which is why this lives here rather than in `src/tests.rs`.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

fn rustctl_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_rustctl"))
}

struct RunningSite {
    child: Child,
    /// stdout and stderr, merged in arrival order (the deprecation warning
    /// is printed to stderr; everything else in the banner goes to stdout).
    lines: Vec<String>,
}

impl RunningSite {
    fn joined(&self) -> String {
        self.lines.join("")
    }

    /// The port from a banner line of the form "http://127.0.0.1:<port>/".
    fn listening_port(&self) -> Option<u16> {
        self.lines.iter().find_map(|line| {
            let after = line.split("http://127.0.0.1:").nth(1)?;
            let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
            digits.parse().ok()
        })
    }

    fn kill(mut self) {
        self.child.kill().ok();
        self.child.wait().ok();
    }
}

fn pump_lines(stream: impl std::io::Read + Send + 'static, tx: mpsc::Sender<String>) {
    thread::spawn(move || {
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    if tx.send(line.clone()).is_err() {
                        break;
                    }
                }
            }
        }
    });
}

/// Spawns `rustctl --site` (or another address/environment supplied by
/// `configure`), waits (up to 5s) for its startup banner's machine-readable
/// line plus a short grace period for the lines printed immediately after
/// it (on either stream), and returns the still-running child together with
/// everything captured so far. The caller is responsible for calling
/// `.kill()` when done with it.
///
/// The grace period exists because the banner is several consecutive
/// `println!`/`eprintln!` calls with no delay between them - waiting only
/// for the first marker line would miss the warnings that follow it.
fn run_site_and_capture(configure: impl FnOnce(&mut Command) -> &mut Command) -> RunningSite {
    let mut command = rustctl_command();
    command
        .arg("--site")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure(&mut command);
    let mut child = command.spawn().expect("failed to launch `rustctl --site`");

    let (tx, rx) = mpsc::channel();
    pump_lines(child.stdout.take().unwrap(), tx.clone());
    pump_lines(child.stderr.take().unwrap(), tx);

    let mut lines = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut saw_banner = false;
    let mut grace_deadline: Option<Instant> = None;
    loop {
        let wait = match grace_deadline {
            Some(gd) if Instant::now() >= gd => break,
            Some(gd) => gd.saturating_duration_since(Instant::now()),
            None if Instant::now() >= deadline => break,
            None => Duration::from_millis(200),
        };
        match rx.recv_timeout(wait) {
            Ok(line) => {
                let is_banner_line = line.contains("hardware_enabled=");
                lines.push(line);
                if is_banner_line {
                    saw_banner = true;
                    // Give both streams a moment to deliver whatever was
                    // printed right after the marker line too.
                    grace_deadline = Some(Instant::now() + Duration::from_millis(400));
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if grace_deadline.is_some() {
                    break;
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    assert!(
        saw_banner,
        "`rustctl --site` did not print its startup banner within the timeout; got: {lines:?}"
    );
    RunningSite { child, lines }
}

#[test]
fn e2e_site_mode_starts_prints_its_banner_and_actually_serves_requests() {
    let site = run_site_and_capture(|command| command.env("RUSTCTL_SITE_ADDR", "127.0.0.1:0"));
    let joined = site.joined();
    assert!(joined.contains("== Site mode =="), "{joined}");
    assert!(joined.contains("hardware_enabled=false"), "{joined}");
    assert!(
        joined.contains("routes=/,/status,/help,/test,/args,/raw,/api"),
        "{joined}"
    );
    // A simulation build (no --features hardware) always warns about it.
    assert!(joined.contains("Simulation build"), "{joined}");

    // The banner is not just printed - the server it describes must really
    // be listening. This is the one thing the in-process concurrency tests
    // (which reimplement run_site()'s accept loop by hand) cannot prove:
    // that the *actual* compiled run_site() wires bind -> banner -> accept
    // loop -> handle_connection together correctly.
    let port = site
        .listening_port()
        .unwrap_or_else(|| panic!("could not find a listening port in banner: {joined}"));
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .expect("the port the banner advertised should actually be accepting connections");
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    stream
        .write_all(b"GET /status HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .unwrap();
    let mut response = Vec::new();
    stream.read_to_end(&mut response).unwrap();
    let response = String::from_utf8(response).unwrap();
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
    assert!(response.contains("\"status\":\"ready\""), "{response}");

    site.kill();
}

#[test]
fn e2e_legacy_api_addr_env_var_still_works_but_warns() {
    let site = run_site_and_capture(|command| {
        command
            .env_remove("RUSTCTL_SITE_ADDR")
            .env("RUSTCTL_API_ADDR", "127.0.0.1:0")
    });
    assert!(
        site.lines
            .iter()
            .any(|line| line.contains("RUSTCTL_API_ADDR is deprecated")),
        "expected a deprecation warning; got: {}",
        site.joined()
    );
    site.kill();
}

#[test]
fn e2e_api_flag_is_rejected_with_a_pointer_to_site() {
    let output = rustctl_command()
        .arg("--api")
        .output()
        .expect("failed to launch `rustctl --api`");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("renamed to '--site'"),
        "stderr was: {stderr}"
    );
}

#[test]
fn e2e_help_flag_documents_site_not_api() {
    let output = rustctl_command()
        .arg("--help")
        .output()
        .expect("failed to launch `rustctl --help`");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--site"), "{stdout}");
    assert!(!stdout.contains("--api"), "{stdout}");
}