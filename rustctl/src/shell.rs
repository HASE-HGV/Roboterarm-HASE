use std::io::{self, BufRead, Write};

use crate::api::{api_command_response, parse_api_request};
use crate::config::{config_from_line, raw_command};
use crate::motion::{execute_position, execute_solution};

pub(crate) fn run_position_loop(api: bool) -> Result<(), Box<dyn std::error::Error>> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    run_position_loop_with_io(stdin.lock(), stdout.lock(), api)
}

pub(crate) fn run_position_loop_with_io<R: BufRead, W: Write>(
    mut reader: R,
    mut writer: W,
    api: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if api {
        writeln!(
            writer,
            "API mode ready. Send one position command per line."
        )?;
    } else {
        writeln!(
            writer,
            "Shell mode. Enter one position command per line, or press Ctrl+D to exit."
        )?;
        writeln!(
            writer,
            "Format: radius_mm base_angle_deg height_mm l1_mm l2_mm steps_per_rev microstep ccw_positive"
        )?;
    }
    loop {
        if !api {
            write!(writer, "shell> ")?;
            writer.flush()?;
        }
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
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
            writeln!(writer, "{response}")?;
        } else {
            match config_from_line(&line).and_then(execute_position) {
                Ok(()) => writeln!(writer, "Command completed.")?,
                Err(error) => writeln!(writer, "Command failed: {error}")?,
            }
        }
    }
    Ok(())
}

pub(crate) fn run_raw_loop() -> Result<(), Box<dyn std::error::Error>> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    run_raw_loop_with_io(stdin.lock(), stdout.lock())
}

pub(crate) fn run_raw_loop_with_io<R: BufRead, W: Write>(
    mut reader: R,
    mut writer: W,
) -> Result<(), Box<dyn std::error::Error>> {
    writeln!(
        writer,
        "Raw mode. Enter: base_deg axis1_deg axis2_deg steps_per_rev microstep ccw_positive"
    )?;
    loop {
        write!(writer, "raw> ")?;
        writer.flush()?;
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        if line.trim().is_empty() {
            continue;
        }
        match raw_command(&line).and_then(|(config, solution)| execute_solution(&config, solution))
        {
            Ok(()) => writeln!(writer, "Command completed.")?,
            Err(error) => writeln!(writer, "Command failed: {error}")?,
        }
    }
    Ok(())
}
