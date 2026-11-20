use std::io::{self, BufRead, Write};

use crate::api::{api_command_response, parse_api_request};
use crate::config::{POSITION_FORMAT, RAW_FORMAT, config_from_line, raw_command};
use crate::motion::{execute_position, execute_solution};
use crate::pretty;

enum InputLine {
    Eof,
    Text(String),
    NotUtf8,
}

fn read_input_line<R: BufRead>(reader: &mut R) -> io::Result<InputLine> {
    let mut bytes = Vec::new();
    if reader.read_until(b'\n', &mut bytes)? == 0 {
        return Ok(InputLine::Eof);
    }
    Ok(match String::from_utf8(bytes) {
        Ok(line) => InputLine::Text(line),
        Err(_) => InputLine::NotUtf8,
    })
}

pub(crate) fn run_position_loop(api: bool) -> Result<(), Box<dyn std::error::Error>> {
    let stdout = io::stdout();
    #[cfg(all(feature = "hardware", target_os = "linux"))]
    {
        crate::motion::reset_stop();
        crate::motion::ensure_ctrlc_handler_installed()?;
        let input = spawn_stdin_reader();
        run_position_loop_with_next_line(|| receive_stdin_line(&input), stdout.lock(), api)
    }
    #[cfg(not(all(feature = "hardware", target_os = "linux")))]
    {
        let stdin = io::stdin();
        run_position_loop_with_io(stdin.lock(), stdout.lock(), api)
    }
}

pub(crate) fn run_position_loop_with_io<R: BufRead, W: Write>(
    mut reader: R,
    writer: W,
    api: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    run_position_loop_with_next_line(|| read_input_line(&mut reader), writer, api)
}

fn run_position_loop_with_next_line<W: Write>(
    mut next_line: impl FnMut() -> io::Result<InputLine>,
    mut writer: W,
    api: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if api {
        writeln!(writer, "{}", pretty::title("API mode ready"))?;
        writeln!(
            writer,
            "{}",
            pretty::info("Send one position command per line.")
        )?;
    } else {
        writeln!(writer, "{}", pretty::title("Shell mode"))?;
        writeln!(
            writer,
            "{}",
            pretty::info("Enter one position command per line, or press Ctrl+D to exit.")
        )?;
        writeln!(
            writer,
            "{}",
            pretty::info(&format!("Format: {POSITION_FORMAT}"))
        )?;
        writeln!(
            writer,
            "{}",
            pretty::info(
                "The last three fields are optional; omit them for an absolute (homed) move."
            )
        )?;
    }
    loop {
        if crate::motion::stop_requested() {
            break;
        }
        if !api {
            write!(writer, "shell> ")?;
            writer.flush()?;
        }
        let line = match next_line()? {
            InputLine::Eof => break,
            InputLine::NotUtf8 => {
                if api {
                    let response = serde_json::json!({"ok": false, "error": {"message": "input is not valid UTF-8"}});
                    writeln!(writer, "{response}")?;
                } else {
                    writeln!(
                        writer,
                        "{}",
                        pretty::warning("Command failed: input is not valid UTF-8")
                    )?;
                }
                continue;
            }
            InputLine::Text(line) => line,
        };
        if crate::motion::stop_requested() {
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
                Ok(()) => writeln!(writer, "{}", pretty::success("Command completed."))?,
                Err(error) => writeln!(
                    writer,
                    "{}",
                    pretty::warning(&format!("Command failed: {error}"))
                )?,
            }
        }
    }
    Ok(())
}

pub(crate) fn run_raw_loop() -> Result<(), Box<dyn std::error::Error>> {
    let stdout = io::stdout();
    #[cfg(all(feature = "hardware", target_os = "linux"))]
    {
        crate::motion::reset_stop();
        crate::motion::ensure_ctrlc_handler_installed()?;
        let input = spawn_stdin_reader();
        run_raw_loop_with_next_line(|| receive_stdin_line(&input), stdout.lock())
    }
    #[cfg(not(all(feature = "hardware", target_os = "linux")))]
    {
        let stdin = io::stdin();
        run_raw_loop_with_io(stdin.lock(), stdout.lock())
    }
}

pub(crate) fn run_raw_loop_with_io<R: BufRead, W: Write>(
    mut reader: R,
    writer: W,
) -> Result<(), Box<dyn std::error::Error>> {
    run_raw_loop_with_next_line(|| read_input_line(&mut reader), writer)
}

fn run_raw_loop_with_next_line<W: Write>(
    mut next_line: impl FnMut() -> io::Result<InputLine>,
    mut writer: W,
) -> Result<(), Box<dyn std::error::Error>> {
    writeln!(writer, "{}", pretty::title("Raw mode"))?;
    writeln!(writer, "{}", pretty::info(&format!("Enter: {RAW_FORMAT}")))?;
    writeln!(
        writer,
        "{}",
        pretty::info("The last three fields are optional; omit them for an absolute (homed) move.")
    )?;
    loop {
        if crate::motion::stop_requested() {
            break;
        }
        write!(writer, "raw> ")?;
        writer.flush()?;
        let line = match next_line()? {
            InputLine::Eof => break,
            InputLine::NotUtf8 => {
                writeln!(
                    writer,
                    "{}",
                    pretty::warning("Command failed: input is not valid UTF-8")
                )?;
                continue;
            }
            InputLine::Text(line) => line,
        };
        if crate::motion::stop_requested() {
            break;
        }
        if line.trim().is_empty() {
            continue;
        }
        match raw_command(&line).and_then(|(config, solution)| execute_solution(&config, solution))
        {
            Ok(()) => writeln!(writer, "{}", pretty::success("Command completed."))?,
            Err(error) => writeln!(
                writer,
                "{}",
                pretty::warning(&format!("Command failed: {error}"))
            )?,
        }
    }
    Ok(())
}

//TODO: maybe optimize???

#[cfg(all(feature = "hardware", target_os = "linux"))]
fn spawn_stdin_reader() -> std::sync::mpsc::Receiver<io::Result<InputLine>> {
    use std::sync::mpsc;

    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let stdin = io::stdin();
        let mut reader = stdin.lock();
        loop {
            let line = read_input_line(&mut reader);
            let eof = matches!(line, Ok(InputLine::Eof));
            if sender.send(line).is_err() || eof {
                break;
            }
        }
    });
    receiver
}

#[cfg(all(feature = "hardware", target_os = "linux"))]
fn receive_stdin_line(
    receiver: &std::sync::mpsc::Receiver<io::Result<InputLine>>,
) -> io::Result<InputLine> {
    use std::sync::mpsc::RecvTimeoutError;
    use std::time::Duration;

    loop {
        if crate::motion::stop_requested() {
            return Ok(InputLine::Eof);
        }
        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(line) => return line,
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return Ok(InputLine::Eof),
        }
    }
}
