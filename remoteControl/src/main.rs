mod traffic_handler;
mod ui;
mod utils;

use std::process::ExitCode;

use traffic_handler::{configure_connection, connect, run_get, run_post};
use utils::{AppError, gap, input, pull_connection_data};

fn run() -> Result<(), AppError> {
    let ip_config = pull_connection_data()?;
    configure_connection(&ip_config.ip, ip_config.mode)?;
    connect(&ip_config)?;

    println!("loop started (quit to exit)");
    gap();

    loop {
        let line = input()?;
        let line = line.trim();

        match line {
            "quit" => {
                break;
            }
            "exit" => {
                break;
            }
            "GET" => {
                run_get()?;
            }
            "POST" => {
                run_post()?;
            }
            _ => {
                return Err(AppError::InvalidRange);
            }
        }
    }

    Ok(())
}

fn main() -> ExitCode {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
