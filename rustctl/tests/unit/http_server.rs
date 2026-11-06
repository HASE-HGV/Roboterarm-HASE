mod server_tests {
    use crate::http_api::{self, HttpListener};
    use std::io;
    use std::net::{SocketAddr, TcpListener, TcpStream};
    use std::sync::mpsc::{self, Sender};

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
