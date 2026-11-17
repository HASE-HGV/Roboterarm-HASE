use std::{
    env, fs,
    io::{self, Read, Write},
    net::{Shutdown, SocketAddr, TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError},
    },
    thread,
    time::{Duration, Instant},
};

use serde_json::{Value, json};

use crate::api::{ApiCommand, api_command_for_request, api_command_response, hardware_enabled};
use crate::control::BusyGuard;
use crate::net;
use crate::pretty;

pub(crate) const DEFAULT_PORT: u16 = 8080;
const MAX_WORKER_THREADS: usize = 64;
const READ_TIMEOUT: Duration = Duration::from_secs(5);
const WRITE_TIMEOUT: Duration = Duration::from_secs(5);
const ACCEPT_ERROR_BACKOFF: Duration = Duration::from_millis(50);
const ACCEPT_ERROR_LOG_INTERVAL: Duration = Duration::from_secs(1);

pub(crate) const PAGE_HTML: &str = include_str!("../web/index.html");
static SERVER_SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);

pub(crate) fn request_server_shutdown() {
    SERVER_SHUTDOWN_REQUESTED.store(true, Ordering::SeqCst);
}

fn clear_server_shutdown() {
    SERVER_SHUTDOWN_REQUESTED.store(false, Ordering::SeqCst);
}

#[derive(Debug)]
pub(crate) struct HttpRequest {
    pub(crate) method: String,
    pub(crate) target: String,
    pub(crate) body: String,
    pub(crate) origin: Option<String>,
    pub(crate) host: Option<String>,
    pub(crate) authorization: Option<String>,
    pub(crate) rustctl_token: Option<String>,
}

#[derive(Debug, PartialEq)]
pub(crate) enum RequestError {
    Timeout,
    Malformed(String),
}

impl RequestError {
    fn message(&self) -> String {
        match self {
            RequestError::Timeout => "request timed out".to_owned(),
            RequestError::Malformed(message) => message.clone(),
        }
    }
}

fn is_timeout(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
    )
}

pub(crate) fn read_http_request(stream: &mut impl Read) -> Result<HttpRequest, RequestError> {
    const HEADER_LIMIT: usize = 16 * 1024;
    let mut buffer = Vec::new();
    let header_end = loop {
        let mut chunk = [0u8; 1024];
        let count = stream.read(&mut chunk).map_err(|error| {
            if is_timeout(&error) {
                RequestError::Timeout
            } else {
                RequestError::Malformed(error.to_string())
            }
        })?;
        if count == 0 {
            return Err(RequestError::Malformed(
                "connection closed before request headers".to_owned(),
            ));
        }
        buffer.extend_from_slice(&chunk[..count]);
        if let Some(end) = buffer.windows(4).position(|window| window == b"\r\n\r\n") {
            break end + 4;
        }
        if buffer.len() > HEADER_LIMIT {
            return Err(RequestError::Malformed(
                "request headers are too large".to_owned(),
            ));
        }
    };
    let (method, target, content_length, origin, host, authorization, rustctl_token) = {
        let text = std::str::from_utf8(&buffer[..header_end])
            .map_err(|_| RequestError::Malformed("invalid HTTP headers".to_owned()))?;
        let mut lines = text.split("\r\n");
        let mut parts = lines
            .next()
            .ok_or_else(|| RequestError::Malformed("missing HTTP request line".to_owned()))?
            .split_whitespace();
        let method = parts
            .next()
            .ok_or_else(|| RequestError::Malformed("missing HTTP method".to_owned()))?;
        let target = parts
            .next()
            .ok_or_else(|| RequestError::Malformed("missing HTTP target".to_owned()))?;
        let version = parts
            .next()
            .ok_or_else(|| RequestError::Malformed("missing HTTP version".to_owned()))?;
        if version != "HTTP/1.1" && version != "HTTP/1.0" {
            return Err(RequestError::Malformed(
                "unsupported HTTP version".to_owned(),
            ));
        }
        let headers: Vec<(&str, &str)> = lines
            .filter_map(|line| line.split_once(':'))
            .map(|(name, value)| (name, value.trim()))
            .collect();
        if headers
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case("transfer-encoding"))
        {
            return Err(RequestError::Malformed(
                "chunked transfer encoding is not supported; send a Content-Length header".to_owned(),
            ));
        }
        let length = headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
            .map(|(_, value)| value.parse::<usize>())
            .transpose()
            .map_err(|_| RequestError::Malformed("invalid Content-Length".to_owned()))?
            .unwrap_or(0);
        let origin = headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("origin"))
            .map(|(_, value)| (*value).to_owned());
        let host = headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("host"))
            .map(|(_, value)| (*value).to_owned());
        let authorization = headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("authorization"))
            .map(|(_, value)| (*value).to_owned());
        let rustctl_token = headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("x-rustctl-token"))
            .map(|(_, value)| (*value).to_owned());
        (
            method.to_owned(),
            target.to_owned(),
            length,
            origin,
            host,
            authorization,
            rustctl_token,
        )
    };
    if content_length > 1024 * 1024 {
        return Err(RequestError::Malformed(
            "request body is too large".to_owned(),
        ));
    }
    while buffer.len() - header_end < content_length {
        let mut chunk = [0u8; 1024];
        let count = stream.read(&mut chunk).map_err(|error| {
            if is_timeout(&error) {
                RequestError::Timeout
            } else {
                RequestError::Malformed(error.to_string())
            }
        })?;
        if count == 0 {
            return Err(RequestError::Malformed(
                "connection closed before request body".to_owned(),
            ));
        }
        buffer.extend_from_slice(&chunk[..count]);
    }
    let body = std::str::from_utf8(&buffer[header_end..header_end + content_length])
        .map_err(|_| RequestError::Malformed("request body is not UTF-8".to_owned()))?;
    Ok(HttpRequest {
        method,
        target,
        body: body.to_owned(),
        origin,
        host,
        authorization,
        rustctl_token,
    })
}

pub(crate) fn write_http_response(
    stream: &mut impl Write,
    status: &str,
    content_type: &str,
    body: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )?;
    stream.flush()?;
    Ok(())
}

fn write_json_response(
    stream: &mut impl Write,
    status: &str,
    body: &Value,
) -> Result<(), Box<dyn std::error::Error>> {
    let body = serde_json::to_string(body)?;
    write_http_response(stream, status, "application/json; charset=utf-8", &body)
}

fn error_json(message: &str) -> Value {
    json!({"ok": false, "error": {"message": message}})
}

pub(crate) fn origin_is_trusted(origin: Option<&str>, host: Option<&str>) -> bool {
    let Some(origin) = origin else {
        return true;
    };
    let Some(host) = host else {
        return false;
    };
    let authority = origin
        .strip_prefix("https://")
        .or_else(|| origin.strip_prefix("http://"));
    match authority {
        Some(authority) => authority.eq_ignore_ascii_case(host),
        None => false,
    }
}

fn read_optional_secret(path: &str) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

pub(crate) fn resolve_token_with(get: impl Fn(&str) -> Option<String>) -> Option<String> {
    for name in ["RUSTCTL_SITE_TOKEN", "RUSTCTL_API_TOKEN"] {
        if let Some(value) = get(name) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_owned());
            }
        }
    }
    for path in [
        "/etc/rustctl/site-token",
        "/etc/rustctl/api-token",
        "/var/run/rustctl/site-token",
        "/var/run/rustctl/api-token",
    ] {
        if let Some(value) = read_optional_secret(path) {
            return Some(value);
        }
    }
    None
}

pub(crate) fn resolve_token() -> Option<String> {
    resolve_token_with(|name| env::var(name).ok())
}

pub(crate) fn request_has_valid_token(request: &HttpRequest, token: &str) -> bool {
    let candidates = [
        request.authorization.as_deref(),
        request.rustctl_token.as_deref(),
    ];
    for value in candidates {
        let Some(value) = value else {
            continue;
        };
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        let bearer = value
            .strip_prefix("Bearer ")
            .or_else(|| value.strip_prefix("bearer "))
            .map(str::trim)
            .unwrap_or(value);
        if bearer == token {
            return true;
        }
    }
    false
}

pub(crate) fn handle_connection_with_shutdown(
    stream: &mut TcpStream,
    shutdown: Option<Sender<()>>,
) -> Result<(), Box<dyn std::error::Error>> {
    stream.set_read_timeout(Some(READ_TIMEOUT))?;
    stream.set_write_timeout(Some(WRITE_TIMEOUT))?;
    handle_request_with_shutdown(stream, shutdown)
}

pub(crate) fn handle_request_with_shutdown(
    stream: &mut (impl Read + Write),
    shutdown: Option<Sender<()>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let request = match read_http_request(stream) {
        Ok(request) => request,
        Err(error) => {
            let status = match error {
                RequestError::Timeout => "408 Request Timeout",
                RequestError::Malformed(_) => "400 Bad Request",
            };
            return write_json_response(stream, status, &error_json(&error.message()));
        }
    };

    let path = request.target.split('?').next().unwrap_or(&request.target);
    if request.method == "GET" && (path == "/" || path == "/index.html") {
        return write_http_response(stream, "200 OK", "text/html; charset=utf-8", PAGE_HTML);
    }

    if request.method == "POST"
        && !origin_is_trusted(request.origin.as_deref(), request.host.as_deref())
    {
        return write_json_response(
            stream,
            "403 Forbidden",
            &error_json("cross-origin request refused"),
        );
    }

    if request.method == "POST" {
        if let Some(token) = resolve_token() {
            if !request_has_valid_token(&request, &token) {
                return write_json_response(
                    stream,
                    "401 Unauthorized",
                    &error_json("missing or invalid API token"),
                );
            }
        }
    }

    if path == "/quit" {
        if request.method != "POST" {
            return write_json_response(
                stream,
                "405 Method Not Allowed",
                &error_json("method not allowed"),
            );
        }
        if !request.body.is_empty() {
            return write_json_response(
                stream,
                "400 Bad Request",
                &error_json("quit request body must be empty"),
            );
        }
        write_json_response(
            stream,
            "200 OK",
            &json!({"ok": true, "status": "shutting_down"}),
        )?;
        crate::motion::request_stop();
        if let Some(shutdown) = shutdown {
            let _ = shutdown.send(());
        }
        return Ok(());
    }

    if path == "/stop" {
        if request.method != "POST" {
            return write_json_response(
                stream,
                "405 Method Not Allowed",
                &error_json("method not allowed"),
            );
        }
        if !request.body.is_empty() {
            return write_json_response(
                stream,
                "400 Bad Request",
                &error_json("stop request body must be empty"),
            );
        }
        crate::motion::request_stop();
        return write_json_response(
            stream,
            "200 OK",
            &json!({"ok": true, "status": "stopped"}),
        );
    }

    match api_command_for_request(&request.method, &request.target, &request.body) {
        Ok(command) => respond_to_command(stream, command),
        Err(error) if error == "method not allowed" => {
            write_json_response(stream, "405 Method Not Allowed", &error_json(&error))
        }
        Err(error) if error == "unknown API route" => {
            write_json_response(stream, "404 Not Found", &error_json(&error))
        }
        Err(error) => write_json_response(stream, "400 Bad Request", &error_json(&error)),
    }
}

fn respond_to_command(
    stream: &mut impl Write,
    command: ApiCommand,
) -> Result<(), Box<dyn std::error::Error>> {
    let is_motion = matches!(command, ApiCommand::Args(_) | ApiCommand::Raw(..));
    if !is_motion {
        return write_json_response(stream, "200 OK", &api_command_response(command));
    }
    match BusyGuard::acquire() {
        Some(_guard) => write_json_response(stream, "200 OK", &api_command_response(command)),
        None => write_json_response(
            stream,
            "409 Conflict",
            &json!({"ok": false, "error": {"message": "arm is busy"}, "busy": true}),
        ),
    }
}

pub(crate) fn resolve_bind_address_with(get: impl Fn(&str) -> Option<String>) -> (String, bool) {
    if let Some(value) = get("RUSTCTL_SITE_ADDR") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return (trimmed.to_owned(), false);
        }
    }
    if let Some(value) = get("RUSTCTL_API_ADDR") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return (trimmed.to_owned(), true);
        }
    }
    (format!("127.0.0.1:{DEFAULT_PORT}"), false)
}

pub(crate) fn resolve_bind_address() -> (String, bool) {
    resolve_bind_address_with(|name| env::var(name).ok())
}

pub(crate) fn banner_lines(
    local_addr: SocketAddr,
    addresses: Vec<net::Address>,
    hostname: Option<String>,
    hardware_enabled: bool,
) -> Vec<String> {
    let port = local_addr.port();
    let is_wildcard = local_addr.ip().is_unspecified();

    let mut urls: Vec<String> = Vec::new();
    if is_wildcard {
        urls.extend(addresses.iter().map(|address| {
            format!(
                "http://{}:{port}/  ({})",
                address.ip, address.interface
            )
        }));
        if let Some(hostname) = &hostname {
            if let Some(url) = net::hostname_url(hostname, port) {
                urls.push(format!("{url}  (hostname, needs mDNS)"));
            }
        }
        if urls.is_empty() {
            urls.push(format!("http://127.0.0.1:{port}/  (loopback only)"));
        }
    } else {
        urls.push(format!("http://{local_addr}/"));
    }

    let mut lines = vec![pretty::title("Site mode")];
    if is_wildcard && addresses.is_empty() {
        lines.push(pretty::warning(
            "No network address found (is the Ethernet cable connected?).",
        ));
    }
    lines.extend(urls.iter().map(|url| pretty::info(url)));
    lines.push(pretty::info(&format!(
        "hardware_enabled={hardware_enabled} | routes=/,/status,/help,/test,/args,/raw,/api,/stop,/quit"
    )));
    if !hardware_enabled {
        lines.push(pretty::warning(
            "Simulation build: no GPIO signals will be sent. Rebuild with --features hardware on the Pi.",
        ));
    }
    lines
}

fn startup_banner(local_addr: SocketAddr) {
    let addresses = net::discover_addresses();
    let hostname = net::read_hostname();
    banner_lines(local_addr, addresses, hostname, hardware_enabled())
        .into_iter()
        .for_each(|line| println!("{line}"));
    if resolve_token().is_some() {
        println!(
            "{}",
            pretty::info("POST routes require a shared secret in Authorization: Bearer <token> or X-Rustctl-Token")
        );
    }
}

pub(crate) fn shutdown_requested(shutdown: &Receiver<()>) -> bool {
    let channel_requested = matches!(
        shutdown.try_recv(),
        Ok(()) | Err(TryRecvError::Disconnected)
    );
    let server_requested = SERVER_SHUTDOWN_REQUESTED
        .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok();
    channel_requested || server_requested
}

fn reap_finished_workers(workers: &mut Vec<thread::JoinHandle<()>>) {
    let mut index = 0;
    while index < workers.len() {
        if workers[index].is_finished() {
            let worker = workers.swap_remove(index);
            let _ = worker.join();
        } else {
            index += 1;
        }
    }
}

pub(crate) trait HttpListener {
    fn accept(&self) -> io::Result<(TcpStream, SocketAddr)>;
}

impl HttpListener for TcpListener {
    fn accept(&self) -> io::Result<(TcpStream, SocketAddr)> {
        TcpListener::accept(self)
    }
}

pub(crate) fn report_connection_result(result: Result<(), Box<dyn std::error::Error>>) {
    if let Err(error) = result {
        eprintln!("{}", pretty::warning(&format!("connection error: {error}")));
    }
}

pub(crate) fn run_api() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let (address, deprecated) = resolve_bind_address();
    run_api_with_address(&address, deprecated)
}

pub(crate) fn run_api_with_address(
    address: &str,
    deprecated: bool,
) -> Result<(), Box<dyn std::error::Error + 'static>> {
    if deprecated {
        eprintln!(
            "{}",
            pretty::warning("RUSTCTL_API_ADDR is deprecated; use RUSTCTL_SITE_ADDR instead.")
        );
    }
    let listener = TcpListener::bind(&address)?;
    listener.set_nonblocking(true)?;
    clear_server_shutdown();
    crate::motion::reset_stop();
    #[cfg(all(feature = "hardware", target_os = "linux"))]
    crate::motion::ensure_ctrlc_handler_installed()?;
    startup_banner(listener.local_addr()?);
    io::stdout().flush()?;
    let (shutdown_tx, shutdown_rx) = mpsc::channel();
    serve_requests(&listener, shutdown_tx, shutdown_rx)
}

pub(crate) fn serve_requests(
    listener: &impl HttpListener,
    shutdown_tx: Sender<()>,
    shutdown_rx: Receiver<()>,
) -> Result<(), Box<dyn std::error::Error + 'static>> {
    let mut workers = Vec::new();
    let mut last_accept_log: Option<Instant> = None;
    let mut suppressed_accept_errors: u64 = 0;
    loop {
        if shutdown_requested(&shutdown_rx) {
            break;
        }
        reap_finished_workers(&mut workers);
        match listener.accept() {
            Ok((mut stream, _)) => {
                if workers.len() >= MAX_WORKER_THREADS {
                    eprintln!(
                        "{}",
                        pretty::warning("too many active client connections; refusing new requests")
                    );
                    let _ = stream.shutdown(Shutdown::Both);
                    continue;
                }
                let shutdown = shutdown_tx.clone();
                workers.push(thread::spawn(move || {
                    report_connection_result(handle_connection_with_shutdown(
                        &mut stream,
                        Some(shutdown),
                    ));
                }));
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                match shutdown_rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(()) | Err(RecvTimeoutError::Disconnected) => break,
                    Err(RecvTimeoutError::Timeout) => {}
                }
            }
            Err(error) => {
                let now = Instant::now();
                if last_accept_log
                    .is_none_or(|at| now.duration_since(at) >= ACCEPT_ERROR_LOG_INTERVAL)
                {
                    let suppressed = match suppressed_accept_errors {
                        0 => String::new(),
                        count => format!(" ({count} similar errors suppressed)"),
                    };
                    eprintln!(
                        "{}",
                        pretty::warning(&format!("API connection failed: {error}{suppressed}"))
                    );
                    last_accept_log = Some(now);
                    suppressed_accept_errors = 0;
                } else {
                    suppressed_accept_errors += 1;
                }
                thread::sleep(ACCEPT_ERROR_BACKOFF);
            }
        }
        reap_finished_workers(&mut workers);
    }
    workers.into_iter().for_each(|worker| {
        let _ = worker.join();
    });
    Ok(())
}