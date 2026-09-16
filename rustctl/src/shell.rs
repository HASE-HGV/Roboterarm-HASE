use std::io::{self, Write};

use crate::api::{api_command_response, parse_api_request};
use crate::config::{config_from_line, raw_command};
use crate::motion::{execute_position, execute_solution};

pub(crate) fn run_position_loop(api: bool) -> Result<(), Box<dyn std::error::Error>> {
    if api {
        println!("API mode ready. Send one position command per line.");
    } else {
        println!("Shell mode. Enter one position command per line, or press Ctrl+D to exit.");
        println!(
            "Format: radius_mm base_angle_deg height_mm l1_mm l2_mm steps_per_rev microstep ccw_positive"
        );
    }
    let stdin = io::stdin();
    loop {
        if !api {
            print!("shell> ");
            io::stdout().flush()?;
        }
        let mut line = String::new();
        if stdin.read_line(&mut line)? == 0 {
            break;
        }
        if line.trim().is_empty() {
            continue;
        }
        if api {
            let response = match parse_api_request(&line, None) {
                Ok(command) => api_command_response(command),
                Err(error) => serde_json::json!({"ok": false, "error": {"message": error}}),
            };
            println!("{response}");
        } else {
            match config_from_line(&line).and_then(execute_position) {
                Ok(()) => println!("Command completed."),
                Err(error) => eprintln!("Command failed: {error}"),
            }
        }
    }
    Ok(())
}

pub(crate) fn run_raw_loop() -> Result<(), Box<dyn std::error::Error>> {
    println!("Raw mode. Enter: base_deg axis1_deg axis2_deg steps_per_rev microstep ccw_positive");
    loop {
        print!("raw> ");
        io::stdout().flush()?;
        let mut line = String::new();
        if io::stdin().read_line(&mut line)? == 0 {
            break;
        }
        if line.trim().is_empty() {
            continue;
        }
        match raw_command(&line).and_then(|(config, solution)| execute_solution(&config, solution))
        {
            Ok(()) => println!("Command completed."),
            Err(error) => eprintln!("Command failed: {error}"),
        }
    }
    Ok(())
}
