use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn rustctl_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_rustctl"))
}

struct RunningSite {
    child: Child,
    log: Arc<Mutex<Vec<String>>>,
}

impl RunningSite {
    fn joined(&self) -> String {
        self.log.lock().unwrap().join("")
    }

    fn listening_port(&self) -> Option<u16> {
        self.log.lock().unwrap().iter().find_map(|line| {
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

fn pump_lines(
    stream: impl std::io::Read + Send + 'static,
    log: Arc<Mutex<Vec<String>>>,
    ready: mpsc::Sender<()>,
) {
    thread::spawn(move || {
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let is_banner_line = line.contains("hardware_enabled=");
                    log.lock().unwrap().push(line.clone());
                    if is_banner_line {
                        let _ = ready.send(());
                    }
                }
            }
        }
    });
}

fn run_site_and_capture(configure: impl FnOnce(&mut Command) -> &mut Command) -> RunningSite {
    let mut command = rustctl_command();
    command
        .arg("--site")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure(&mut command);
    let mut child = command.spawn().expect("failed to launch `rustctl --site`");

    let log = Arc::new(Mutex::new(Vec::new()));
    let (ready_tx, ready_rx) = mpsc::channel();
    pump_lines(
        child.stdout.take().unwrap(),
        Arc::clone(&log),
        ready_tx.clone(),
    );
    pump_lines(child.stderr.take().unwrap(), Arc::clone(&log), ready_tx);

    ready_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("`rustctl --site` did not print its startup banner within the timeout");

    RunningSite { child, log }
}

#[test]
fn e2e_legacy_api_addr_env_var_still_works_but_warns() {
    let site = run_site_and_capture(|command| {
        command
            .env_remove("RUSTCTL_SITE_ADDR")
            .env("RUSTCTL_API_ADDR", "127.0.0.1:0")
    });
    assert!(
        site.log
            .lock()
            .unwrap()
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

#[test]
fn e2e_help_flag_documents_the_start_position() {
    let output = rustctl_command()
        .arg("--help")
        .output()
        .expect("failed to launch `rustctl --help`");
    let stdout = String::from_utf8_lossy(&output.stdout);
    for field in ["start_base_deg", "start_axis1_deg", "start_axis2_deg"] {
        assert!(
            stdout.contains(field),
            "help must mention {field}: {stdout}"
        );
    }
    assert!(stdout.contains("relative"), "{stdout}");
}

fn request(port: u16, raw: &[u8]) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("failed to connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    stream.write_all(raw).unwrap();
    let mut response = Vec::new();
    stream.read_to_end(&mut response).unwrap();
    String::from_utf8(response).unwrap()
}

fn json_post(port: u16, path: &str, body: &str) -> String {
    let raw = format!(
        "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    request(port, raw.as_bytes())
}

#[test]
fn e2e_site_serves_the_control_page_with_both_themes() {
    let site = run_site_and_capture(|command| command.env("RUSTCTL_SITE_ADDR", "127.0.0.1:0"));
    let port = site.listening_port().unwrap_or_else(|| {
        panic!(
            "could not find a listening port in banner: {}",
            site.joined()
        )
    });

    let response = request(port, b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n");
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
    assert!(response.contains("Content-Type: text/html"), "{response}");
    assert!(response.contains("<h1>Roboterarm HASE</h1>"), "{response}");
    assert!(
        response.contains(r#"data-theme="win95""#),
        "the page must ship the win95 theme: {response}"
    );
    assert!(
        response.contains(r#"id="theme""#),
        "the page must offer a theme toggle: {response}"
    );
    assert!(response.contains("CSS-less"), "{response}");
    for field in ["start_base_deg", "start_axis1_deg", "start_axis2_deg"] {
        assert!(response.contains(field), "the page must offer {field}");
    }

    site.kill();
}

#[test]
fn e2e_site_rejects_a_partial_start_position() {
    let site = run_site_and_capture(|command| command.env("RUSTCTL_SITE_ADDR", "127.0.0.1:0"));
    let port = site.listening_port().unwrap_or_else(|| {
        panic!(
            "could not find a listening port in banner: {}",
            site.joined()
        )
    });

    let response = json_post(
        port,
        "/args",
        r#"{"radius_mm":100,"base_angle_deg":0,"height_mm":50,"l1_mm":200,"l2_mm":200,"steps_per_rev":200,"microstep":16,"ccw_positive":true,"start_base_deg":5}"#,
    );
    assert!(
        response.starts_with("HTTP/1.1 400 Bad Request\r\n"),
        "{response}"
    );
    assert!(response.contains("all of start_base_deg"), "{response}");

    site.kill();
}

#[test]
fn e2e_site_help_documents_the_start_position() {
    let site = run_site_and_capture(|command| command.env("RUSTCTL_SITE_ADDR", "127.0.0.1:0"));
    let port = site.listening_port().unwrap_or_else(|| {
        panic!(
            "could not find a listening port in banner: {}",
            site.joined()
        )
    });

    let response = request(port, b"GET /help HTTP/1.1\r\nHost: localhost\r\n\r\n");
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
    for field in ["start_base_deg", "start_axis1_deg", "start_axis2_deg"] {
        assert!(
            response.contains(field),
            "help must mention {field}: {response}"
        );
    }

    site.kill();
}

#[test]
fn e2e_self_test_endpoint_reports_every_runtime_check_passing() {
    let site = run_site_and_capture(|command| command.env("RUSTCTL_SITE_ADDR", "127.0.0.1:0"));
    let port = site.listening_port().unwrap_or_else(|| {
        panic!(
            "could not find a listening port in banner: {}",
            site.joined()
        )
    });

    let response = request(port, b"GET /test HTTP/1.1\r\nHost: localhost\r\n\r\n");
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
    assert!(
        response.contains("\"status\":\"tests_completed\""),
        "{response}"
    );
    assert!(response.contains("\"failed\":0"), "{response}");
    assert!(
        response.contains("runtime_test_start_position_is_relative"),
        "the start position check must run in the self-test: {response}"
    );

    site.kill();
}

#[test]
fn e2e_shell_mode_executes_a_relative_command() {
    let mut child = rustctl_command()
        .arg("--shell")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to launch `rustctl --shell`");
    {
        let mut stdin = child.stdin.take().unwrap();

        stdin
            .write_all(b"100 0 50 200 200 200 16 1 10 20 30\n")
            .unwrap();
    }
    let output = child.wait_with_output().expect("shell mode did not exit");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Shell mode"), "{stdout}");
    assert!(stdout.contains("Command completed"), "{stdout}");
    assert!(stdout.contains("Start position"), "{stdout}");
    assert!(stdout.contains("Relative travel"), "{stdout}");
}

#[test]
fn e2e_raw_mode_executes_a_relative_command() {
    let mut child = rustctl_command()
        .arg("--raw")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to launch `rustctl --raw`");
    {
        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(b"0 25 30 200 16 1 10 20 30\n").unwrap();
    }
    let output = child.wait_with_output().expect("raw mode did not exit");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Raw mode"), "{stdout}");
    assert!(stdout.contains("Command completed"), "{stdout}");
    assert!(stdout.contains("Start position"), "{stdout}");
}

#[test]
fn e2e_shell_mode_rejects_a_partial_start_position_and_keeps_reading() {
    let mut child = rustctl_command()
        .arg("--shell")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to launch `rustctl --shell`");
    {
        let mut stdin = child.stdin.take().unwrap();
        stdin
            .write_all(b"100 0 50 200 200 200 16 1 10 20\n")
            .unwrap();
        stdin.write_all(b"100 0 50 200 200 200 16 1\n").unwrap();
    }
    let output = child.wait_with_output().expect("shell mode did not exit");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Command failed"), "{stdout}");
    assert!(stdout.contains("8 or 11"), "{stdout}");

    assert_eq!(stdout.matches("Command completed").count(), 1, "{stdout}");
}
