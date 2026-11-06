use super::*;

#[test]
fn a_domain_label() {
    announce_domain("HTTP protocol and page");
}

#[test]
fn http_status_request_returns_json_success() {
    let response = http_exchange(b"GET /status HTTP/1.1\r\nHost: localhost\r\n\r\n");
    let response = String::from_utf8(response).unwrap();
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(response.contains("Content-Type: application/json"));
    assert!(response.contains("\"status\":\"ready\""));
}

#[test]
fn http_post_args_returns_success() {
    let _serial = busy_test_lock();
    let body = br#"{"radius_mm":100,"base_angle_deg":0,"height_mm":50,"l1_mm":200,"l2_mm":200,"steps_per_rev":200,"microstep":16,"ccw_positive":true}"#;
    let request = format!(
        "POST /args HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        std::str::from_utf8(body).unwrap()
    );
    let response = String::from_utf8(http_exchange(request.as_bytes())).unwrap();
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(response.contains("\"mode\":\"args\""));
}

#[test]
fn http_post_args_with_a_start_position_succeeds() {
    let _serial = busy_test_lock();
    let response = http_post(
        "/args",
        r#"{"radius_mm":100,"base_angle_deg":0,"height_mm":50,"l1_mm":200,"l2_mm":200,"steps_per_rev":200,"microstep":16,"ccw_positive":true,"start_base_deg":5,"start_axis1_deg":10,"start_axis2_deg":15}"#,
    );
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
    assert!(response.contains("\"mode\":\"args\""), "{response}");
}

#[test]
fn http_post_raw_with_a_start_position_succeeds() {
    let _serial = busy_test_lock();
    let response = http_post(
        "/raw",
        r#"{"base_deg":0,"axis1_deg":25,"axis2_deg":30,"steps_per_rev":200,"microstep":16,"ccw_positive":true,"start_base_deg":5,"start_axis1_deg":10,"start_axis2_deg":15}"#,
    );
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
    assert!(response.contains("\"mode\":\"raw\""), "{response}");
}

#[test]
fn http_post_args_with_a_partial_start_position_is_rejected() {
    let response = http_post(
        "/args",
        r#"{"radius_mm":100,"base_angle_deg":0,"height_mm":50,"l1_mm":200,"l2_mm":200,"steps_per_rev":200,"microstep":16,"ccw_positive":true,"start_base_deg":5}"#,
    );
    assert!(
        response.starts_with("HTTP/1.1 400 Bad Request\r\n"),
        "{response}"
    );
    assert!(response.contains("all of start_base_deg"), "{response}");
}

#[test]
fn http_get_help_documents_the_start_position() {
    let response =
        String::from_utf8(http_exchange(b"GET /help HTTP/1.1\r\nHost: localhost\r\n\r\n")).unwrap();
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
    for field in ["start_base_deg", "start_axis1_deg", "start_axis2_deg"] {
        assert!(response.contains(field), "help must mention {field}: {response}");
    }
}

#[test]
fn http_routes_return_expected_error_statuses() {
    let not_found = http_exchange(b"GET /missing HTTP/1.1\r\nHost: localhost\r\n\r\n");
    assert!(
        String::from_utf8(not_found)
            .unwrap()
            .starts_with("HTTP/1.1 404 Not Found\r\n")
    );

    let method = http_exchange(b"PUT /status HTTP/1.1\r\nHost: localhost\r\n\r\n");
    assert!(
        String::from_utf8(method)
            .unwrap()
            .starts_with("HTTP/1.1 405 Method Not Allowed\r\n")
    );

    let bad_json = http_exchange(
        b"POST /api HTTP/1.1\r\nHost: localhost\r\nContent-Length: 8\r\n\r\nnot-json",
    );
    assert!(
        String::from_utf8(bad_json)
            .unwrap()
            .starts_with("HTTP/1.1 400 Bad Request\r\n")
    );
}

#[test]
fn http_quit_route_checks_method_and_empty_body() {
    let get = String::from_utf8(http_exchange(
        b"GET /quit HTTP/1.1\r\nHost: localhost\r\n\r\n",
    ))
    .unwrap();
    assert!(get.starts_with("HTTP/1.1 405 Method Not Allowed\r\n"), "{get}");

    let post_body = String::from_utf8(http_exchange(
        b"POST /quit HTTP/1.1\r\nHost: localhost\r\nContent-Length: 1\r\n\r\nx",
    ))
    .unwrap();
    assert!(post_body.starts_with("HTTP/1.1 400 Bad Request\r\n"), "{post_body}");
    assert!(post_body.contains("quit request body must be empty"), "{post_body}");

    let post = String::from_utf8(http_exchange(
        b"POST /quit HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0\r\n\r\n",
    ))
    .unwrap();
    assert!(post.starts_with("HTTP/1.1 200 OK\r\n"), "{post}");
    assert!(post.contains("shutting_down"), "{post}");
}

#[test]
fn http_malformed_requests_return_bad_request() {
    let response = http_exchange(b"not an HTTP request\r\n\r\n");
    let response = String::from_utf8(response).unwrap();
    assert!(response.starts_with("HTTP/1.1 400 Bad Request\r\n"));
    assert!(response.contains("unsupported HTTP version"));
}

#[test]
fn http_rejects_unsupported_version_and_invalid_content_length() {
    let version = http_exchange(b"GET /status HTTP/2.0\r\nHost: localhost\r\n\r\n");
    assert!(
        String::from_utf8(version)
            .unwrap()
            .contains("unsupported HTTP version")
    );

    let length =
        http_exchange(b"POST /api HTTP/1.1\r\nHost: localhost\r\nContent-Length: nope\r\n\r\n");
    assert!(
        String::from_utf8(length)
            .unwrap()
            .contains("invalid Content-Length")
    );
}

#[test]
fn http_request_reader_rejects_chunked_transfer_encoding() {
    let mut chunked = Cursor::new(
        b"POST /args HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: chunked\r\n\r\n2\r\n{}\r\n0\r\n\r\n"
            .to_vec(),
    );
    match read_http_request(&mut chunked) {
        Err(RequestError::Malformed(message)) => {
            assert!(message.contains("chunked"), "{message}")
        }
        other => panic!("expected a Malformed(chunked) error, got {other:?}"),
    }

    let mut mixed_case = Cursor::new(
        b"POST /args HTTP/1.1\r\ntransfer-ENCODING: gzip, chunked\r\nContent-Length: 2\r\n\r\n{}"
            .to_vec(),
    );
    assert!(matches!(
        read_http_request(&mut mixed_case),
        Err(RequestError::Malformed(message)) if message.contains("chunked")
    ));
}

#[test]
fn http_request_reader_still_parses_content_length_bodies() {
    let mut request = Cursor::new(
        b"POST /args HTTP/1.1\r\nHost: localhost\r\nContent-Length: 2\r\n\r\n{}".to_vec(),
    );
    let parsed = read_http_request(&mut request).unwrap();
    assert_eq!(parsed.method, "POST");
    assert_eq!(parsed.target, "/args");
    assert_eq!(parsed.body, "{}");
}

#[test]
fn http_chunked_post_is_answered_with_bad_request_naming_chunked_encoding() {
    let response = http_exchange(
        b"POST /args HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: chunked\r\n\r\n2\r\n{}\r\n0\r\n\r\n",
    );
    let response = String::from_utf8(response).unwrap();
    assert!(response.starts_with("HTTP/1.1 400 Bad Request\r\n"));
    assert!(response.contains("chunked transfer encoding is not supported"));
}

#[test]
fn http_request_reader_rejects_non_utf8_headers_and_bodies() {
    let mut invalid_header = Cursor::new(b"GET /status HTTP/1.1\r\nX-Name: \xff\r\n\r\n".to_vec());
    assert_eq!(
        read_http_request(&mut invalid_header).unwrap_err(),
        RequestError::Malformed("invalid HTTP headers".to_owned())
    );

    let mut invalid_body = Cursor::new(
        b"POST /api HTTP/1.1\r\nContent-Length: 1\r\n\r\n\xff".to_vec(),
    );
    assert_eq!(
        read_http_request(&mut invalid_body).unwrap_err(),
        RequestError::Malformed("request body is not UTF-8".to_owned())
    );
}

#[test]
fn http_request_reader_reports_non_timeout_io_errors_as_malformed() {
    assert_eq!(
        read_http_request(&mut FailingReader).unwrap_err(),
        RequestError::Malformed("synthetic read failure".to_owned())
    );
}

#[test]
fn http_request_reader_reports_body_io_errors_and_short_request_lines() {
    let mut body_error = FailAfterHeader(Cursor::new(
        b"POST /api HTTP/1.1\r\nContent-Length: 1\r\n\r\n".to_vec(),
    ));
    assert_eq!(
        read_http_request(&mut body_error).unwrap_err(),
        RequestError::Malformed("synthetic body read failure".to_owned())
    );

    for (raw, expected) in [
        (&b"\r\n\r\n"[..], "missing HTTP method"),
        (&b"GET\r\n\r\n"[..], "missing HTTP target"),
        (&b"GET /status\r\n\r\n"[..], "missing HTTP version"),
    ] {
        let mut cursor = Cursor::new(raw.to_vec());
        assert_eq!(
            read_http_request(&mut cursor).unwrap_err(),
            RequestError::Malformed(expected.to_owned())
        );
    }
}

#[test]
fn http_response_writer_propagates_write_and_flush_errors() {
    exercise_writer_failures(|writer| {
        crate::http_api::write_http_response(writer, "200 OK", "text/plain", "body")
    });
    exercise_writer_failures(|writer| {
        let mut stream = RequestThenFailingWriter {
            request: Cursor::new(
                b"POST /quit HTTP/1.1\r\nContent-Length: 0\r\n\r\n".to_vec(),
            ),
            output: writer,
        };
        crate::http_api::handle_request_with_shutdown(&mut stream, None)
    });
}

#[test]
fn origin_absent_is_always_trusted() {
    assert!(origin_is_trusted(None, None));
    assert!(origin_is_trusted(None, Some("localhost:5000")));
}

#[test]
fn origin_matching_host_is_trusted_regardless_of_scheme_or_case() {
    assert!(origin_is_trusted(
        Some("http://localhost:5000"),
        Some("localhost:5000")
    ));
    assert!(origin_is_trusted(
        Some("https://LOCALHOST:5000"),
        Some("localhost:5000")
    ));
    assert!(origin_is_trusted(
        Some("http://192.168.1.50:5000"),
        Some("192.168.1.50:5000")
    ));
}

#[test]
fn origin_mismatched_host_is_rejected() {
    assert!(!origin_is_trusted(
        Some("http://evil.example"),
        Some("localhost:5000")
    ));
    assert!(!origin_is_trusted(
        Some("http://localhost:5000"),
        Some("localhost:5001")
    ));
}

#[test]
fn origin_without_scheme_or_without_host_header_is_rejected() {
    assert!(!origin_is_trusted(
        Some("localhost:5000"),
        Some("localhost:5000")
    ));
    assert!(!origin_is_trusted(Some("http://localhost:5000"), None));
    assert!(!origin_is_trusted(Some("null"), Some("localhost:5000")));
}

#[test]
fn req_oversized_headers_are_rejected() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        read_http_request(&mut stream)
    });
    let mut client = TcpStream::connect(address).unwrap();
    client
        .write_all(b"GET /status HTTP/1.1\r\nHost: localhost\r\n")
        .unwrap();
    client.write_all(&vec![b'x'; 17 * 1024]).unwrap();
    let result = server.join().unwrap();
    match result {
        Err(RequestError::Malformed(message)) => {
            assert!(message.contains("too large"), "{message}")
        }
        other => panic!("expected a Malformed(too large) error, got {other:?}"),
    }
}

#[test]
fn req_content_length_over_one_mib_is_rejected_before_reading_the_body() {
    let response = String::from_utf8(http_exchange(
        b"POST /api HTTP/1.1\r\nHost: localhost\r\nContent-Length: 5000000\r\n\r\n",
    ))
    .unwrap();
    assert!(
        response.starts_with("HTTP/1.1 400 Bad Request\r\n"),
        "{response}"
    );
    assert!(response.contains("too large"));
}

#[test]
fn req_body_delivered_across_multiple_reads_is_reassembled() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let body = br#"{"radius_mm":100,"base_angle_deg":0,"height_mm":50,"l1_mm":200,"l2_mm":200,"steps_per_rev":200,"microstep":16,"ccw_positive":true}"#;
    let head = format!(
        "POST /args HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n",
        body.len()
    );
    let split_at = body.len() / 2;
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        read_http_request(&mut stream)
    });
    let mut client = TcpStream::connect(address).unwrap();
    client.write_all(head.as_bytes()).unwrap();
    client.write_all(&body[..split_at]).unwrap();
    thread::sleep(Duration::from_millis(80));
    client.write_all(&body[split_at..]).unwrap();
    let request = server.join().unwrap().expect("request should parse");
    assert_eq!(request.method, "POST");
    assert_eq!(request.body, std::str::from_utf8(body).unwrap());
}

#[test]
fn req_body_closing_early_is_reported_as_malformed() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        read_http_request(&mut stream)
    });
    let mut client = TcpStream::connect(address).unwrap();
    client
        .write_all(
            b"POST /args HTTP/1.1\r\nHost: localhost\r\nContent-Length: 100\r\n\r\n{\"partial",
        )
        .unwrap();
    client.shutdown(Shutdown::Both).unwrap();
    let result = server.join().unwrap();
    match result {
        Err(RequestError::Malformed(message)) => {
            assert!(message.contains("before request body"), "{message}")
        }
        other => panic!("expected a Malformed(before request body) error, got {other:?}"),
    }
}

#[test]
fn req_body_read_can_time_out_independently_of_the_header_read() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_millis(150)))
            .unwrap();
        read_http_request(&mut stream)
    });
    let mut client = TcpStream::connect(address).unwrap();
    client
        .write_all(
            b"POST /args HTTP/1.1\r\nHost: localhost\r\nContent-Length: 100\r\n\r\n{\"partial",
        )
        .unwrap();
    let result = server.join().unwrap();
    assert_eq!(result.unwrap_err(), RequestError::Timeout);
    drop(client);
}

#[test]
fn req_read_http_request_times_out_without_data() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_millis(150)))
            .unwrap();
        read_http_request(&mut stream)
    });
    let _client = TcpStream::connect(address).unwrap();
    let result = server.join().unwrap();
    assert_eq!(result.unwrap_err(), RequestError::Timeout);
}

#[test]
fn req_handle_request_maps_timeout_to_408() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_millis(150)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        crate::http_api::handle_request_with_shutdown(&mut stream, None).unwrap();
    });
    let mut client = TcpStream::connect(address).unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let mut response = Vec::new();
    client.read_to_end(&mut response).unwrap();
    server.join().unwrap();
    let response = String::from_utf8(response).unwrap();
    assert!(
        response.starts_with("HTTP/1.1 408 Request Timeout\r\n"),
        "{response}"
    );
    assert!(response.contains("request timed out"));
}

#[test]
fn con_second_connection_served_while_first_is_idle() {
    let address = spawn_accept_loop();
    let _idle = TcpStream::connect(address).unwrap();

    let mut b = TcpStream::connect(address).unwrap();
    b.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    b.write_all(b"GET /status HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .unwrap();
    let response = read_full_response(&mut b);
    assert!(
        response.starts_with("HTTP/1.1 200 OK\r\n"),
        "a second client must be served promptly while the first is idle: {response}"
    );
}

#[test]
fn con_accept_loop_survives_abrupt_client_disconnect() {
    let address = spawn_accept_loop();
    {
        let mut a = TcpStream::connect(address).unwrap();
        a.write_all(b"GET /status HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .unwrap();
        a.shutdown(Shutdown::Both).ok();
    }
    thread::sleep(Duration::from_millis(150));
    let mut b = TcpStream::connect(address).unwrap();
    b.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    b.write_all(b"GET /status HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .unwrap();
    let response = read_full_response(&mut b);
    assert!(
        response.starts_with("HTTP/1.1 200 OK\r\n"),
        "the accept loop must keep serving requests after another client disconnects abruptly: {response}"
    );
}

#[test]
fn con_cross_origin_post_is_rejected() {
    let body = br#"{"radius_mm":100,"base_angle_deg":0,"height_mm":50,"l1_mm":200,"l2_mm":200,"steps_per_rev":200,"microstep":16,"ccw_positive":true}"#;
    let request = format!(
        "POST /args HTTP/1.1\r\nHost: localhost:5000\r\nOrigin: http://evil.example\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        std::str::from_utf8(body).unwrap()
    );
    let response = String::from_utf8(http_exchange(request.as_bytes())).unwrap();
    assert!(
        response.starts_with("HTTP/1.1 403 Forbidden\r\n"),
        "{response}"
    );
    assert!(response.contains("cross-origin request refused"));
}

#[test]
fn con_same_origin_post_is_allowed() {
    let _serial = busy_test_lock();
    let body = br#"{"radius_mm":100,"base_angle_deg":0,"height_mm":50,"l1_mm":200,"l2_mm":200,"steps_per_rev":200,"microstep":16,"ccw_positive":true}"#;
    let request = format!(
        "POST /args HTTP/1.1\r\nHost: localhost:5000\r\nOrigin: http://localhost:5000\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        std::str::from_utf8(body).unwrap()
    );
    let response = String::from_utf8(http_exchange(request.as_bytes())).unwrap();
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
    assert!(response.contains("\"mode\":\"args\""));
}

#[test]
fn con_get_ignores_origin_header() {
    let request =
        b"GET /status HTTP/1.1\r\nHost: localhost:5000\r\nOrigin: http://evil.example\r\n\r\n";
    let response = String::from_utf8(http_exchange(request)).unwrap();
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
}

#[test]
fn con_busy_arm_rejects_concurrent_motion_request_with_409() {
    let _serial = busy_test_lock();
    let body = br#"{"radius_mm":100,"base_angle_deg":0,"height_mm":50,"l1_mm":200,"l2_mm":200,"steps_per_rev":200,"microstep":16,"ccw_positive":true}"#;
    let request = format!(
        "POST /args HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        std::str::from_utf8(body).unwrap()
    );

    let guard = BusyGuard::acquire().unwrap();
    let response = String::from_utf8(http_exchange(request.as_bytes())).unwrap();
    assert!(
        response.starts_with("HTTP/1.1 409 Conflict\r\n"),
        "{response}"
    );
    assert!(response.contains("arm is busy"));
    assert!(response.contains("\"busy\":true"));
    drop(guard);

    let response = String::from_utf8(http_exchange(request.as_bytes())).unwrap();
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
}

#[test]
fn con_status_reports_busy_field() {
    let _serial = busy_test_lock();
    let idle = String::from_utf8(http_exchange(
        b"GET /status HTTP/1.1\r\nHost: localhost\r\n\r\n",
    ))
    .unwrap();
    assert!(idle.contains("\"busy\":false"), "{idle}");

    let guard = BusyGuard::acquire().unwrap();
    let busy = String::from_utf8(http_exchange(
        b"GET /status HTTP/1.1\r\nHost: localhost\r\n\r\n",
    ))
    .unwrap();
    assert!(busy.contains("\"busy\":true"), "{busy}");
    drop(guard);
}

#[test]
fn page_html_constant_is_self_contained_with_no_external_resources() {
    let lower = PAGE_HTML.to_ascii_lowercase();
    assert!(
        !lower.contains("stylesheet"),
        "page must not link an external stylesheet"
    );
    assert!(
        !lower.contains(".css"),
        "page must not reference a CSS file"
    );
    assert!(
        !PAGE_HTML.contains("http://") && !PAGE_HTML.contains("https://"),
        "page must be self-contained: no external URLs"
    );
    assert!(!lower.contains("innerhtml"), "page must not use innerHTML");
    assert!(!lower.contains("eval("), "page must not use eval()");
    assert!(!lower.contains("document.write"));
    for element in PAGE_HTML.lines() {
        assert!(
            !element.trim_start().starts_with("style="),
            "page must not use inline style attributes: {element}"
        );
    }
}

#[test]
fn page_styles_are_scoped_to_the_styled_themes() {
    let css = page_style_block();
    let selectors: Vec<&str> = css
        .lines()
        .map(str::trim)
        .filter(|line| line.ends_with('{') && line.contains('{') && !line.starts_with('@'))
        .collect();
    assert!(
        selectors.len() > 10,
        "expected a full set of theme rules, got {selectors:?}"
    );
    for selector in &selectors {
        assert!(
            selector.starts_with("html[data-theme=\"win95\"]")
                || selector.starts_with("html[data-theme=\"winxp\"]")
                || selector.starts_with("html[data-theme=\"win10\"]")
                || selector.starts_with("html[data-theme=\"macos\"]")
                || selector.starts_with("html:not([data-theme=\"plain\"])"),
            "every rule must be scoped to a styled theme so CSS-less stays unstyled: {selector}"
        );
    }
    for selector in &selectors {
        assert!(
            !selector.contains("[data-theme=\"plain\"]")
                || selector.starts_with("html:not([data-theme=\"plain\"])"),
            "the plain theme must not be styled at all: {selector}"
        );
    }
}

#[test]
fn page_offers_a_theme_selector_for_all_themes() {
    assert!(
        PAGE_HTML.contains(r#"data-theme="win95""#),
        "the page must declare the win95 theme as its default"
    );
    assert!(
        PAGE_HTML.contains(r#"id="theme""#),
        "page must have a theme selector"
    );
    assert!(PAGE_HTML.contains("<select id=\"theme\""));
    for token in ["THEME_KEY", "applyTheme", "localStorage", "rustctl-theme"] {
        assert!(PAGE_HTML.contains(token), "theme persistence is missing {token}");
    }
    for theme in ["Windows 95", "Windows XP", "Windows 10", "macOS", "CSS-less"] {
        assert!(PAGE_HTML.contains(theme), "the selector must offer {theme}");
    }
}

#[test]
fn page_form_fields_match_the_json_api_field_names() {
    for field in [
        "radius_mm",
        "base_angle_deg",
        "height_mm",
        "l1_mm",
        "l2_mm",
        "steps_per_rev",
        "microstep",
        "ccw_positive",
        "base_deg",
        "axis1_deg",
        "axis2_deg",
        "start_base_deg",
        "start_axis1_deg",
        "start_axis2_deg",
    ] {
        assert!(
            PAGE_HTML.contains(&format!("name=\"{field}\"")),
            "page is missing a field for {field}"
        );
    }
}

#[test]
fn page_offers_a_start_position_in_both_motion_forms() {
    for (id, path) in [("args", "/args"), ("raw", "/raw")] {
        let start = PAGE_HTML
            .find(&format!("id=\"{id}\""))
            .unwrap_or_else(|| panic!("page must contain the {id} form"));
        let end = PAGE_HTML[start..]
            .find("</form>")
            .map(|offset| start + offset)
            .expect("form must be closed");
        let form = &PAGE_HTML[start..end];
        assert!(form.contains(&format!("data-path=\"{path}\"")), "{id}");
        for field in ["start_base_deg", "start_axis1_deg", "start_axis2_deg"] {
            assert!(
                form.contains(&format!("name=\"{field}\"")),
                "the {id} form is missing {field}"
            );
        }
    }
}

#[test]
fn page_omits_an_all_zero_start_position_from_the_request() {
    assert!(
        PAGE_HTML.contains(r#"el.name.startsWith("start_")"#),
        "the page must drop all-zero start angles so the controller stays homed"
    );
}

#[test]
fn page_root_serves_html_with_expected_headers() {
    let response =
        String::from_utf8(http_exchange(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")).unwrap();
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
    assert!(response.contains("Content-Type: text/html"));
    assert!(response.contains("Connection: close"));
    assert!(response.contains("<h1>Roboterarm HASE</h1>"));
}

#[test]
fn page_index_html_alias_serves_identical_content() {
    let root = http_exchange(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n");
    let alias = http_exchange(b"GET /index.html HTTP/1.1\r\nHost: localhost\r\n\r\n");
    assert_eq!(root, alias);
}

#[test]
fn page_query_string_is_ignored() {
    let response = String::from_utf8(http_exchange(
        b"GET /?debug=1 HTTP/1.1\r\nHost: localhost\r\n\r\n",
    ))
    .unwrap();
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
}

#[test]
fn page_other_unknown_paths_still_404() {
    let response = String::from_utf8(http_exchange(
        b"GET /favicon.ico HTTP/1.1\r\nHost: localhost\r\n\r\n",
    ))
    .unwrap();
    assert!(
        response.starts_with("HTTP/1.1 404 Not Found\r\n"),
        "{response}"
    );
}

#[test]
fn page_post_to_root_is_not_treated_as_the_page_route() {
    let response = String::from_utf8(http_exchange(
        b"POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0\r\n\r\n",
    ))
    .unwrap();
    assert!(!response.starts_with("HTTP/1.1 200 OK\r\n"));
}

#[test]
fn http_request_reader_accepts_a_body_at_the_one_mib_limit() {
    let body = vec![b'x'; 1024 * 1024];
    let mut request = format!(
        "POST /api HTTP/1.1\r\nContent-Length: {}\r\n\r\n",
        body.len()
    )
    .into_bytes();
    request.extend_from_slice(&body);
    let parsed = read_http_request(&mut Cursor::new(request)).unwrap();
    assert_eq!(parsed.body.len(), 1024 * 1024);
}

#[test]
fn http_position_rejects_unsupported_microstep_with_structured_error() {
    let body = args_request_with_motor_settings(200, 3);
    let request = format!(
        "POST /args HTTP/1.1\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    let response = String::from_utf8(http_exchange(request.as_bytes())).unwrap();
    assert!(
        response.starts_with("HTTP/1.1 400 Bad Request\r\n")
            && response.contains(r#""ok":false"#)
            && response.contains("microstep must be one of")
    );
}

#[test]
fn http_api_rejects_invalid_json_with_structured_error() {
    let response = String::from_utf8(http_exchange(
        b"POST /api HTTP/1.1\r\nContent-Length: 8\r\n\r\nnot-json",
    ))
    .unwrap();
    assert!(
        response.starts_with("HTTP/1.1 400 Bad Request\r\n")
            && response.contains(r#""ok":false"#)
            && response.contains("invalid JSON")
    );
}

