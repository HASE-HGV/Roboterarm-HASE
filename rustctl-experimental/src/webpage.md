This specification outlines the migration of the rustctl web server from a focused API to a production-ready local service, including a public-facing (local network) landing page and expanded CLI functionality.

  Specification: rustctl Web Server & Local Network Host

  1. Overview

  The goal is to transform the rustctl binary from a CLI-first tool with an optional API into a local-network-accessible web server. This involves:
  - Adding a static frontend page accessible via a web browser.
  - Enhancing the CLI to describe where the server is running.
  - Retaining all existing API functionality while ensuring "catch-all" behavior for unknown requests.

  2. CLI & Configuration Changes

  The following changes will be applied to rustctl/src/cli.rs:

  Flag Migration

  - Remove: --api
  - Add: --site
    - Action: Runs run_api().
    - Output: When executed, the terminal must print the binding address (e.g., Running on http://192.168.1.50:5000) to allow immediate local network access.

  Logic for get_mode

  The match statement in get_mode will be updated to:
  Some("--site") => run_api(),

  3. Web Server & Network Configuration

  Binding

  - Current: Defaults to 127.0.0.1:5000.
  - Requirement: Must bind to 0.0.0.0:5000 (or ::1:5000) to be reachable over Ethernet/Wi-Fi on the Raspberry Pi.
  - Environment Variable: RUSTCTL_API_ADDR will be honored for overriding this.

  Route Definitions

  The server will maintain the following routes:

| Method | Path | Description |
| :--- | :--- | :--- |
| GET | / | New: Barebones HTML page |
| GET | /status | System status and hardware info |
| GET | /help | API documentation |
| GET | /test | Run internal hardware/software tests |
| POST | /args | Execute motion by XYZ coordinates |
| POST | /raw | Execute motion by raw angles |
| POST | /api | Generic JSON-based API entry |



  4. New Features

  Barebones HTML Page

  - Path: /
  - Content: A minimal, non-styled HTML page served as plain text or text/html.
  - Purpose: Connectivity verification for the local network.
  - Implementation: Added to handle_connection or handled as a special case in api_command_for_request.

  "Catch-All" API Handling

  - Every request not matching a known route will return a 404 Not Found or 400 Bad Request with a JSON body describing the error.
  - Ensure all POST requests without a valid body return a structured error rather than crashing the listener.

  5. Testing Strategy

  To ensure "Maximum Test Coverage," the following test suites must be implemented/expanded:

  Unit Tests (tests.rs)

  - Response Parsing: Verify that all JSON payloads (Args, Raw, Status) are parsed correctly.
  - Constraint Validation: Ensure microstep and steps_per_rev values are within valid bounds.
  - Error Mapping: Verify that incorrect commands return the expected Err(String) variants.

  Integration Tests

  - HTTP Loopback: Mock TcpStream to simulate full request/response cycles for /status and /help.
  - Boundary Testing: Verify Content-Length limits and large payload handling.

  Hardware/Integration Tests

  - Mock Driver: Verify that ApiCommand::Args correctly translates to execute_position calls without actual hardware attached (simulated).

  6. Implementation Plan

  1. Update cli.rs: Rename --api to --site and update get_mode.
  2. Update http_api.rs:
     - Add 0.0.0.0 binding logic.
     - Add GET / handler to serve the barebones HTML.
     - Update api_command_for_request to include the root path.
  3. Add Network Detection: Logic to detect local IP (e.g., host command or getifaddrs crate) to display in the --site output.
  4. Coverage Audit: Run cargo test and verify runtime_test_report() passes.