use std::io::{self, BufRead, Write};

use crate::config::{MotionConfig, config_from_position};
use crate::http_api::run_api;
use crate::motion::execute_position;
use crate::pretty;
use crate::shell::{run_position_loop, run_raw_loop};

pub(crate) fn prompt_position() -> Result<MotionConfig, Box<dyn std::error::Error>> {
    let stdin: io::Stdin = io::stdin();
    let stdout: io::Stdout = io::stdout();
    prompt_position_with_io(stdin.lock(), stdout.lock())
}

pub(crate) fn prompt_position_with_io<R: BufRead, W: Write>(
    mut reader: R,
    mut writer: W,
) -> Result<MotionConfig, Box<dyn std::error::Error>> {
    writeln!(writer, "{}", pretty::title("CLI mode"))?;
    writeln!(
        writer,
        "{}",
        pretty::info("Timing is fixed at 1083 us / 500 us.")
    )?;
    let mut values: Vec<String> = Vec::with_capacity(8);
    for (label, example) in [
        ("Target radius X (mm)", "100"),
        ("Base angle Y (degrees)", "0"),
        ("Target height Z (mm)", "50"),
        ("Arm 1 length (mm)", "200"),
        ("Arm 2 length (mm)", "200"),
        ("Motor steps per revolution", "200"),
        ("Driver microstep resolution", "16"),
        ("CCW positive? (1 = yes, 0 = no)", "1"),
    ] {
        write!(writer, "{}", pretty::prompt(label, example))?;
        writer.flush()?;
        let mut input: String = String::new();
        reader.read_line(&mut input)?;
        values.push(input.trim().to_owned());
    }
    let refs: Vec<&str> = values.iter().map(String::as_str).collect();
    config_from_position(&refs)
}

pub(crate) fn print_help(program: &str) {
    println!("{}", pretty::help(program));
}

pub(crate) fn get_mode(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    match args.get(1).map(String::as_str) {
        Some("--cli") => execute_position(prompt_position()?),
        Some("--shell") => run_position_loop(false),
        Some("--api") => run_api(),
        Some("--raw") => run_raw_loop(),
        Some("--help") | None => {
            print_help(args.first().map(String::as_str).unwrap_or("rustctl"));
            Ok(())
        }
        Some(mode) => Err(format!("Unknown mode '{mode}'. Use --help for usage.").into()),
    }
}
