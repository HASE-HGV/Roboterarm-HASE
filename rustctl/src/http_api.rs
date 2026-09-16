use std::{
    env,
    io::{self, Read, Write},
    net::{TcpListener, TcpStream},
};

use serde_json::{Value, json};

use crate::api::{api_command_for_request, api_command_response, hardware_enabled};

const API_PORT: u16 = 5000;

fn read_http_request(stream: &mut TcpStream) -> Result<(String, String, String), String> {
    const HEADER_LIMIT: usize = 16 * 1024;
    let mut buffer = Vec::new();
    let header_end = loop {
        let mut chunk = [0u8; 1024];
        let count = stream.read(&mut chunk).map_err(|error| error.to_string())?;
        if count == 0 {
            return Err("connection closed before request headers".to_owned());
        }
        buffer.extend_from_slice(&chunk[..count]);
        if let Some(end) = buffer.windows(4).position(|window| window == b"\r\n\r\n") {
            break end + 4;
        }
        if buffer.len() > HEADER_LIMIT {
            return Err("request headers are too large".to_owned());
        }
    };
    let (method, target, content_length) = {
        let text =
            std::str::from_utf8(&buffer[..header_end]).map_err(|_| "invalid HTTP headers")?;
        let mut lines = text.split("\r\n");
        let mut parts = lines
            .next()
            .ok_or("missing HTTP request line")?
            .split_whitespace();
        let method = parts.next().ok_or("missing HTTP method")?;
        let target = parts.next().ok_or("missing HTTP target")?;
        let version = parts.next().ok_or("missing HTTP version")?;
        if version != "HTTP/1.1" && version != "HTTP/1.0" {
            return Err("unsupported HTTP version".to_owned());
        }
        let length = lines
            .filter_map(|line| line.split_once(':'))
            .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
            .map(|(_, value)| value.trim().parse::<usize>())
            .transpose()
            .map_err(|_| "invalid Content-Length".to_owned())?
            .unwrap_or(0);
        (method.to_owned(), target.to_owned(), length)
    };
    if content_length > 1024 * 1024 {
        return Err("request body is too large".to_owned());
    }
    while buffer.len() - header_end < content_length {
        let mut chunk = [0u8; 1024];
        let count = stream.read(&mut chunk).map_err(|error| error.to_string())?;
        if count == 0 {
            return Err("connection closed before request body".to_owned());
        }
        buffer.extend_from_slice(&chunk[..count]);
    }
    let body = std::str::from_utf8(&buffer[header_end..header_end + content_length])
        .map_err(|_| "request body is not UTF-8")?;
    Ok((method, target, body.to_owned()))
}

fn write_http_response(
    stream: &mut TcpStream,
    status: &str,
    body: &Value,
) -> Result<(), Box<dyn std::error::Error>> {
    let body = serde_json::to_string(body)?;
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )?;
    stream.flush()?;
    Ok(())
}

fn handle_connection(mut stream: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    match read_http_request(&mut stream) {
        Ok((method, target, body)) => match api_command_for_request(&method, &target, &body) {
            Ok(command) => {
                write_http_response(&mut stream, "200 OK", &api_command_response(command))?
            }
            Err(error) if error == "method not allowed" => write_http_response(
                &mut stream,
                "405 Method Not Allowed",
                &json!({"ok": false, "error": {"message": error}}),
            )?,
            Err(error) if error == "unknown API route" => write_http_response(
                &mut stream,
                "404 Not Found",
                &json!({"ok": false, "error": {"message": error}}),
            )?,
            Err(error) => write_http_response(
                &mut stream,
                "400 Bad Request",
                &json!({"ok": false, "error": {"message": error}}),
            )?,
        },
        Err(error) => write_http_response(
            &mut stream,
            "400 Bad Request",
            &json!({"ok": false, "error": {"message": error}}),
        )?,
    }
    Ok(())
}

pub(crate) fn run_api() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let address = env::var("RUSTCTL_API_ADDR").unwrap_or_else(|_| format!("127.0.0.1:{API_PORT}"));
    let listener = TcpListener::bind(&address)?;
    println!(
        "API listening on http://{address} hardware_enabled={} routes=/status,/help,/test,/args,/raw",
        hardware_enabled()
    );
    io::stdout().flush()?;
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => handle_connection(stream)?,
            Err(error) => eprintln!("API connection failed: {error}"),
        }
    }
    Ok(())
}
