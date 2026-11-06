use std::io::{BufRead, Cursor, Read, Write};

use std::net::{Ipv4Addr, Shutdown, SocketAddr, TcpListener, TcpStream};

use std::sync::Mutex;

use std::thread;

use std::time::Duration;

use crate::cli::{get_mode, print_help, prompt_position_with_io};

use crate::control::{BusyGuard, is_busy};

use crate::http_api::{
    PAGE_HTML, RequestError, banner_lines,
    origin_is_trusted, read_http_request, resolve_bind_address_with,
};

use crate::net::{
    Address, discover_addresses, hostname_url, rank_addresses, rank_ip_addresses,
    read_hostname, read_hostname_from,
};

use crate::shell::{run_position_loop_with_io, run_raw_loop_with_io};

use crate::config::{MotionConfig, StartPosition, validate_motor_settings};

use crate::motion::{joint_deltas, plan_report, step_plan};

fn announce_domain(label: &str) {
    use std::io::Write;
    let mut stdout = std::io::stdout().lock();
    writeln!(stdout, "\n{label}").expect("write test domain heading");
    stdout.flush().expect("flush test domain heading");
}

static BUSY_TEST_LOCK: Mutex<()> = Mutex::new(());

fn busy_test_lock() -> std::sync::MutexGuard<'static, ()> {
    BUSY_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-6
}

fn position_line(target: [f64; 3], start: Option<[f64; 3]>) -> String {
    let mut fields = vec![
        target[0].to_string(),
        target[1].to_string(),
        target[2].to_string(),
        "200".to_owned(),
        "200".to_owned(),
        "200".to_owned(),
        "1".to_owned(),
        "1".to_owned(),
    ];
    if let Some(start) = start {
        fields.extend(start.map(|value| value.to_string()));
    }
    fields.join(" ")
}

fn http_exchange(request: &[u8]) -> Vec<u8> {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        crate::http_api::handle_connection_with_shutdown(&mut stream, None).unwrap();
    });

    let mut client = TcpStream::connect(address).unwrap();
    client.write_all(request).unwrap();
    client.shutdown(Shutdown::Write).unwrap();
    let mut response = Vec::new();
    client.read_to_end(&mut response).unwrap();
    server.join().unwrap();
    response
}

struct FailAtOperation {
    fail_at: usize,
    operations: usize,
}

impl FailAtOperation {
    fn tick(&mut self) -> std::io::Result<()> {
        if self.operations == self.fail_at {
            return Err(std::io::Error::other("synthetic output failure"));
        }
        self.operations += 1;
        Ok(())
    }
}

impl Write for FailAtOperation {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        self.tick()?;
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.tick()
    }
}

struct RequestThenFailingWriter<'a> {
    request: Cursor<Vec<u8>>,
    output: &'a mut FailAtOperation,
}

impl Read for RequestThenFailingWriter<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        self.request.read(buffer)
    }
}

impl Write for RequestThenFailingWriter<'_> {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        self.output.write(buffer)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.output.flush()
    }
}

fn exercise_writer_failures(
    mut operation: impl FnMut(&mut FailAtOperation) -> Result<(), Box<dyn std::error::Error>>,
) {
    let mut saw_error = false;
    for fail_at in 0..128 {
        let mut writer = FailAtOperation {
            fail_at,
            operations: 0,
        };
        match operation(&mut writer) {
            Ok(()) => return,
            Err(_) => saw_error = true,
        }
    }
    assert!(saw_error, "the writer was never asked to write or flush");
    panic!("writer operation sweep did not reach a successful run");
}

struct FailingBufReader;

impl Read for FailingBufReader {
    fn read(&mut self, _buffer: &mut [u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("synthetic input failure"))
    }
}

impl BufRead for FailingBufReader {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        Err(std::io::Error::other("synthetic input failure"))
    }

    fn consume(&mut self, _amount: usize) {}
}

struct FailingStartReader(Cursor<Vec<u8>>);

impl Read for FailingStartReader {
    fn read(&mut self, _buffer: &mut [u8]) -> std::io::Result<usize> {
        if self.0.position() == self.0.get_ref().len() as u64 {
            Err(std::io::Error::other("synthetic input failure"))
        } else {
            self.0.read(_buffer)
        }
    }
}

impl BufRead for FailingStartReader {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        if self.0.position() == self.0.get_ref().len() as u64 {
            Err(std::io::Error::other("synthetic input failure"))
        } else {
            self.0.fill_buf()
        }
    }

    fn consume(&mut self, amount: usize) {
        self.0.consume(amount);
    }
}

fn http_post(path: &str, body: &str) -> String {
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    String::from_utf8(http_exchange(request.as_bytes())).unwrap()
}

struct FailingReader;

impl Read for FailingReader {
    fn read(&mut self, _buffer: &mut [u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("synthetic read failure"))
    }
}

struct FailAfterHeader(Cursor<Vec<u8>>);

impl Read for FailAfterHeader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if self.0.position() == self.0.get_ref().len() as u64 {
            Err(std::io::Error::other("synthetic body read failure"))
        } else {
            self.0.read(buffer)
        }
    }
}

fn spawn_accept_loop() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    thread::spawn(move || {
        for stream in listener.incoming() {
            if let Ok(stream) = stream {
                thread::spawn(move || {
                    let mut stream = stream;
                    let _ = crate::http_api::handle_connection_with_shutdown(&mut stream, None);
                });
            }
        }
    });
    address
}

fn read_full_response(stream: &mut TcpStream) -> String {
    let mut response = Vec::new();
    stream.read_to_end(&mut response).unwrap();
    String::from_utf8(response).unwrap()
}

fn page_style_block() -> &'static str {
    let start = PAGE_HTML.find("<style>").expect("page must ship its own CSS");
    let body = &PAGE_HTML[start + "<style>".len()..];
    let end = body.find("</style>").expect("the style block must be closed");
    &body[..end]
}

fn addr(interface: &str, ip: [u8; 4]) -> Address {
    Address {
        interface: interface.to_owned(),
        ip: Ipv4Addr::from(ip),
    }
}

fn ik_round_trip_matches(radius: f64, base: f64, height: f64, l1: f64, l2: f64) -> bool {
    let Ok(solution) = ik_angles_3d_deg(radius, base, height, l1, l2) else {
        return false;
    };
    let (actual_radius, actual_height) =
        forward_r_z_mm(solution.theta1_deg, solution.theta2_deg, l1, l2);
    approx(actual_radius, radius)
        && approx(actual_height, height)
        && approx(solution.theta_base_deg, base)
}

fn args_request_with_motor_settings(steps_per_rev: u64, microstep: u64) -> String {
    format!(
        r#"{{"command":"args","radius_mm":100,"base_angle_deg":0,"height_mm":50,"l1_mm":200,"l2_mm":200,"steps_per_rev":{steps_per_rev},"microstep":{microstep},"ccw_positive":true}}"#
    )
}

fn raw_request_with_motor_settings(steps_per_rev: u64, microstep: u64) -> String {
    format!(
        r#"{{"command":"raw","base_deg":0,"axis1_deg":25,"axis2_deg":30,"steps_per_rev":{steps_per_rev},"microstep":{microstep},"ccw_positive":true}}"#
    )
}

fn args_request_without(field: &str) -> String {
    let mut request = serde_json::json!({
        "command": "args",
        "radius_mm": 1,
        "base_angle_deg": 2,
        "height_mm": 3,
        "l1_mm": 4,
        "l2_mm": 5,
        "steps_per_rev": 200,
        "microstep": 8,
        "ccw_positive": true,
    });
    request.as_object_mut().unwrap().remove(field);
    request.to_string()
}

fn raw_request_without(field: &str) -> String {
    let mut request = serde_json::json!({
        "command": "raw",
        "base_deg": 1,
        "axis1_deg": 2,
        "axis2_deg": 3,
        "steps_per_rev": 200,
        "microstep": 8,
        "ccw_positive": true,
    });
    request.as_object_mut().unwrap().remove(field);
    request.to_string()
}
