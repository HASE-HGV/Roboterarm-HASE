use std::{
    env,
    io::{self, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    thread,
    time::Duration,
};

use serde_json::{Value, json};

use crate::api::{ApiCommand, api_command_for_request, api_command_response, hardware_enabled};
use crate::control::BusyGuard;
use crate::net;
use crate::pretty;

pub(crate) const DEFAULT_PORT: u16 = 8080;
const READ_TIMEOUT: Duration = Duration::from_secs(5);
const WRITE_TIMEOUT: Duration = Duration::from_secs(5);

pub(crate) const PAGE_HTML: &str = include_str!("../web/index.html");

#[derive(Debug)]
pub(crate) struct HttpRequest {
    pub(crate) method: String,
    pub(crate) target: String,
    pub(crate) body: String,
    pub(crate) origin: Option<String>,
    pub(crate) host: Option<String>,
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
    let (method, target, content_length, origin, host) = {
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
        (method.to_owned(), target.to_owned(), length, origin, host)
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

pub(crate) fn handle_connection(mut stream: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    stream.set_read_timeout(Some(READ_TIMEOUT))?;
    stream.set_write_timeout(Some(WRITE_TIMEOUT))?;
    handle_request(&mut stream)
}

pub(crate) fn handle_request(
    stream: &mut (impl Read + Write),
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
    (format!("0.0.0.0:{DEFAULT_PORT}"), false)
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
    let is_loopback = local_addr.ip().is_loopback();

    let mut urls: Vec<String> = Vec::new();
    if is_wildcard {
        for address in &addresses {
            urls.push(format!(
                "http://{}:{port}/  ({})",
                address.ip, address.interface
            ));
        }
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
    for url in &urls {
        lines.push(pretty::info(url));
    }
    lines.push(pretty::info(&format!(
        "hardware_enabled={hardware_enabled} | routes=/,/status,/help,/test,/args,/raw,/api"
    )));
    if !hardware_enabled {
        lines.push(pretty::warning(
            "Simulation build: no GPIO signals will be sent. Rebuild with --features hardware on the Pi.",
        ));
    }
    if !is_loopback {
        lines.push(pretty::warning(
            "No login: anyone who can reach this address can move the arm.",
        ));
    }
    lines
}

fn startup_banner(local_addr: SocketAddr) {
    let addresses = net::discover_addresses();
    let hostname = net::read_hostname();
    for line in banner_lines(local_addr, addresses, hostname, hardware_enabled()) {
        println!("{line}");
    }
}

pub(crate) fn run_api() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let (address, deprecated) = resolve_bind_address();
    if deprecated {
        eprintln!(
            "{}",
            pretty::warning("RUSTCTL_API_ADDR is deprecated; use RUSTCTL_SITE_ADDR instead.")
        );
    }
    let listener = TcpListener::bind(&address)?;
    startup_banner(listener.local_addr()?);
    io::stdout().flush()?;
    // Each connection gets its own thread so one slow or misbehaving client
    // (finding F-1) or one that resets its connection (finding F-2) cannot
    // block or kill every other client - the old --api's single sequential
    // loop did both. A connection's error is logged here, never propagated:
    // nothing a client sends can stop this loop.
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(move || {
                    if let Err(error) = handle_connection(stream) {
                        eprintln!("{}", pretty::warning(&format!("connection error: {error}")));
                    }
                });
            }
            Err(error) => eprintln!(
                "{}",
                pretty::warning(&format!("API connection failed: {error}"))
            ),
        }
    }
    Ok(())
}
