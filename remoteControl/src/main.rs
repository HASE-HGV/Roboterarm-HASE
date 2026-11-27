mod traffic_handler;
mod ui;
mod utils;

use std::process::ExitCode;

use traffic_handler::configure_connection;
use utils::{AppError, pull_connection_data};

fn run() -> Result<(), AppError> {
    let ip_config = pull_connection_data()?;

    configure_connection(&ip_config.ip, ip_config.mode)
}

fn main() -> ExitCode {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
