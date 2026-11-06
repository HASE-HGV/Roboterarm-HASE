mod server_tests {
    use crate::http_api::{self, HttpListener};
    use std::io::{self, Read, Write};
    use std::net::{SocketAddr, TcpListener, TcpStream};
    use std::sync::mpsc::{self, Sender};
    use std::thread;
    use std::time::{Duration, Instant};

    #[test]
    fn a_domain_label() {
        use std::io::Write;
        let mut stdout = std::io::stdout().lock();
        writeln!(stdout, "\nHTTP server lifecycle").expect("write test domain heading");
        stdout.flush().expect("flush test domain heading");
    }

    #[test]
    fn server_loop_stops_for_a_queued_shutdown() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let (shutdown_tx, shutdown_rx) = mpsc::channel();
        shutdown_tx.send(()).unwrap();
        http_api::serve_requests(&listener, shutdown_tx, shutdown_rx).unwrap();
    }

    #[test]
    fn real_server_handles_requests_and_shuts_down_after_quit() {
        let reservation = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = reservation.local_addr().unwrap();
        drop(reservation);

        let server_address = address.to_string();
        let server = thread::spawn(move || {
            http_api::run_api_with_address(&server_address, false)
                .map_err(|error| error.to_string())
        });
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut status = loop {
            match TcpStream::connect(address) {
                Ok(stream) => break stream,
                Err(_) if Instant::now() < deadline => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("server did not start listening: {error}"),
            }
        };
        status
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        status
            .write_all(b"GET /status HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .unwrap();
        let mut response = String::new();
        status.read_to_string(&mut response).unwrap();
        assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");

        let mut quit = TcpStream::connect(address).unwrap();
        quit.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        quit.write_all(
            b"POST /quit HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0\r\n\r\n",
        )
        .unwrap();
        let mut response = String::new();
        quit.read_to_string(&mut response).unwrap();
        assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
        assert!(response.contains("shutting_down"), "{response}");
        server.join().unwrap().unwrap();
    }

    #[test]
    fn run_api_uses_the_legacy_address_and_reports_bind_errors() {
        let site_address = std::env::var_os("RUSTCTL_SITE_ADDR");
        let legacy_address = std::env::var_os("RUSTCTL_API_ADDR");
        // The test runner is serialized, and run_api reads the values before returning.
        unsafe {
            std::env::set_var("RUSTCTL_SITE_ADDR", " ");
            std::env::set_var("RUSTCTL_API_ADDR", "invalid-address");
        }
        let result = http_api::run_api();
        unsafe {
            match site_address {
                Some(value) => std::env::set_var("RUSTCTL_SITE_ADDR", value),
                None => std::env::remove_var("RUSTCTL_SITE_ADDR"),
            }
            match legacy_address {
                Some(value) => std::env::set_var("RUSTCTL_API_ADDR", value),
                None => std::env::remove_var("RUSTCTL_API_ADDR"),
            }
        }

        assert!(result.is_err());
    }

    #[test]
    fn shutdown_poll_accepts_messages_and_disconnected_senders() {
        let (sender, receiver) = mpsc::channel();
        assert!(!http_api::shutdown_requested(&receiver));
        sender.send(()).unwrap();
        assert!(http_api::shutdown_requested(&receiver));

        let (sender, receiver) = mpsc::channel();
        drop(sender);
        assert!(http_api::shutdown_requested(&receiver));
    }

    struct FailingListener(Sender<()>);

    impl HttpListener for FailingListener {
        fn accept(&self) -> io::Result<(TcpStream, SocketAddr)> {
            self.0.send(()).unwrap();
            Err(io::Error::other("synthetic accept failure"))
        }
    }

    #[test]
    fn server_loop_survives_a_listener_error_and_logs_worker_errors() {
        let (shutdown_tx, shutdown_rx) = mpsc::channel();
        http_api::serve_requests(
            &FailingListener(shutdown_tx.clone()),
            shutdown_tx,
            shutdown_rx,
        )
        .unwrap();
        http_api::report_connection_result(Err(
            io::Error::other("synthetic connection failure").into(),
        ));
    }
}
