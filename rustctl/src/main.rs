use std::{
    env,
    io::{self, Read, Write},
    net::{TcpListener, TcpStream},
};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

include!("bresenham.rs");
include!("api.rs");

const API_PORT: u16 = 5000;

#[cfg(all(feature = "hardware", target_os = "linux"))]
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

#[cfg(all(feature = "hardware", target_os = "linux"))]
use rppal::gpio::{Gpio, OutputPin};

const GEAR_RATIO: f64 = 16.0;
const TOTAL_TIME_US: u64 = 1083;
const PULSE_T_US: u64 = 500;
const HARDWARE_OVERHEAD_US: u64 = 83;
const NUM_AXES: usize = 3;

#[cfg(all(feature = "hardware", target_os = "linux"))]
const PIN_AXIS1: (u8, u8) = (17, 27);
#[cfg(all(feature = "hardware", target_os = "linux"))]
const PIN_AXIS2: (u8, u8) = (22, 23);
#[cfg(all(feature = "hardware", target_os = "linux"))]
const PIN_BASE: (u8, u8) = (24, 25);
#[cfg(all(feature = "hardware", target_os = "linux"))]
const PIN_ENDEFFECTOR: (u8, u8) = (5, 6);

macro_rules! debug_invariant {
    ($cond:expr) => {
        #[cfg(debug_assertions)]
        {
            if !($cond) {
                panic!("debug invariant violated: {}", stringify!($cond));
            }
        }
    };
    ($cond:expr, $($dump:expr),+ $(,)?) => {
        #[cfg(debug_assertions)]
        {
            if !($cond) {
                $( dbg!(&$dump); )+
                panic!("debug invariant violated: {}", stringify!($cond));
            }
        }
    };
}

include!("kinematics.rs");

#[derive(Debug, Clone, Copy, PartialEq)]
struct MotionConfig {
    total_time_us: u64,
    pulse_t_us: u64,
    x_mm: f64,
    y_mm: f64,
    z_mm: f64,
    l1_mm: f64,
    l2_mm: f64,
    steps_per_rev: u64,
    microstep: u64,
    ccw_positive: bool,
}

fn config_from_position(values: &[&str]) -> Result<MotionConfig, Box<dyn std::error::Error>> {
    if values.len() != 8 {
        return Err("Expected: radius_mm base_angle_deg height_mm l1_mm l2_mm steps_per_rev microstep ccw_positive".into());
    }
    Ok(MotionConfig {
        total_time_us: TOTAL_TIME_US,
        pulse_t_us: PULSE_T_US,
        x_mm: values[0].parse()?,
        y_mm: values[1].parse()?,
        z_mm: values[2].parse()?,
        l1_mm: values[3].parse()?,
        l2_mm: values[4].parse()?,
        steps_per_rev: values[5].parse()?,
        microstep: values[6].parse()?,
        ccw_positive: values[7].parse::<u8>()? != 0,
    })
}

fn prompt_position() -> Result<MotionConfig, Box<dyn std::error::Error>> {
    println!("CLI mode (timing is fixed at 1083 µs / 500 µs)");
    let mut values = Vec::with_capacity(8);
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
        print!("{label} [{example}]: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        values.push(input.trim().to_owned());
    }
    let refs: Vec<&str> = values.iter().map(String::as_str).collect();
    config_from_position(&refs)
}

fn config_from_line(line: &str) -> Result<MotionConfig, Box<dyn std::error::Error>> {
    let values: Vec<&str> = line.split_whitespace().collect();
    config_from_position(&values)
}

fn print_help(program: &str) {
    println!("Roboterarm controller\n");
    println!("Usage: {program} --cli | --shell | --raw | --api | --help");
    println!("\nModes:");
    println!("  --cli   Prompt for one XYZ position and execute it.");
    println!(
        "  --shell Repeatedly read radius/angle/height commands from an interactive input loop."
    );
    println!(
        "  --raw   Repeatedly read raw angles: base_deg axis1_deg axis2_deg steps_per_rev microstep ccw_positive."
    );
    println!("  --api   Read API commands from stdin and write responses to stdout.");
    println!("  --help  Show this guide.");
    println!("\nPosition command format:");
    println!(
        "  radius_mm base_angle_deg height_mm l1_mm l2_mm steps_per_rev microstep ccw_positive"
    );
    println!("Timing is fixed: total period = {TOTAL_TIME_US} µs, pulse width = {PULSE_T_US} µs.");
    println!("\nPC testing:");
    println!("  cargo run -- --cli");
    println!("  cargo run -- --shell");
    println!("  cargo test");
    println!("\nRaspberry Pi hardware:");
    println!("  cargo build --release --features hardware");
    println!("  sudo ./target/release/rustctl --shell");
    println!(
        "Commands are processed until EOF or Ctrl+C. Linux builds access GPIO; other platforms simulate motion and present it in a table view."
    );
}

fn raw_command(line: &str) -> Result<(MotionConfig, ArmSolution), Box<dyn std::error::Error>> {
    let values: Vec<&str> = line.split_whitespace().collect();
    if values.len() != 6 {
        return Err(
            "Expected: base_deg axis1_deg axis2_deg steps_per_rev microstep ccw_positive".into(),
        );
    }
    let solution = ArmSolution {
        theta_base_deg: values[0].parse()?,
        theta1_deg: values[1].parse()?,
        theta2_deg: values[2].parse()?,
        z_eff_mm: 0.0,
    };
    let config = MotionConfig {
        total_time_us: TOTAL_TIME_US,
        pulse_t_us: PULSE_T_US,
        x_mm: 0.0,
        y_mm: 0.0,
        z_mm: 0.0,
        l1_mm: 1.0,
        l2_mm: 1.0,
        steps_per_rev: values[3].parse()?,
        microstep: values[4].parse()?,
        ccw_positive: values[5].parse::<u8>()? != 0,
    };
    Ok((config, solution))
}

#[cfg(all(feature = "hardware", target_os = "linux"))]
struct StepperMotor {
    step_pin: OutputPin,
    dir_pin: OutputPin,
}

#[cfg(all(feature = "hardware", target_os = "linux"))]
impl StepperMotor {
    fn new(gpio: &Gpio, step: u8, dir: u8) -> Result<Self, rppal::gpio::Error> {
        Ok(Self {
            step_pin: gpio.get(step)?.into_output(),
            dir_pin: gpio.get(dir)?.into_output(),
        })
    }

    fn set_direction_ccw(&mut self, ccw: bool) {
        if ccw {
            self.dir_pin.set_high();
        } else {
            self.dir_pin.set_low();
        }
    }

    fn pulse_high(&mut self) {
        self.step_pin.set_high();
    }

    fn pulse_low(&mut self) {
        self.step_pin.set_low();
    }

    fn reset(&mut self) {
        self.step_pin.set_low();
        self.dir_pin.set_low();
    }
}

#[cfg(all(feature = "hardware", target_os = "linux"))]
fn run_motion(
    motors: &mut [StepperMotor; NUM_AXES],
    steps: [i64; NUM_AXES],
    pulse_t_us: u64,
    overhead_us: u64,
    terminate: &AtomicBool,
) -> [i64; NUM_AXES] {
    let expected: [i64; NUM_AXES] = std::array::from_fn(|i| steps[i].abs());
    let mut stepped = [0i64; NUM_AXES];

    let pulse = Duration::from_micros(pulse_t_us);
    let overhead = Duration::from_micros(overhead_us);

    for pulses in MultiAxisPlanner::new(steps) {
        if terminate.load(Ordering::SeqCst) {
            break;
        }

        for i in 0..NUM_AXES {
            if pulses[i] {
                motors[i].pulse_high();
            }
        }
        thread::sleep(pulse);

        for i in 0..NUM_AXES {
            if pulses[i] {
                motors[i].pulse_low();
                stepped[i] += 1;
            }
        }
        thread::sleep(pulse);

        if overhead_us > 0 {
            thread::sleep(overhead);
        }
    }

    if !terminate.load(Ordering::SeqCst) {
        debug_assert_eq!(stepped, expected, "planner under-/over-stepped an axis");
        debug_invariant!(
            stepped.iter().sum::<i64>() == expected.iter().sum::<i64>(),
            stepped,
            expected
        );
    }

    stepped
}

fn print_plan(s: &ArmSolution, steps: &[i64; NUM_AXES]) {
    println!("\n=== Kinematics ===");
    println!(
        "Base angle: {:.3}°, Axis 1: {:.3}°, Axis 2: {:.3}°, effective Z: {:.3} mm",
        s.theta_base_deg, s.theta1_deg, s.theta2_deg, s.z_eff_mm
    );
    println!(
        "Target steps (16:1 gearbox): Base: {}, Axis 1: {}, Axis 2: {}",
        steps[2], steps[0], steps[1]
    );
}

fn execute_solution(
    config: &MotionConfig,
    solution: ArmSolution,
) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(not(all(feature = "hardware", target_os = "linux")))]
    let _ = config.ccw_positive;

    let overhead_us = match overhead_sleep_us(config.total_time_us, config.pulse_t_us) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };

    let steps: [i64; NUM_AXES] = [
        deg_to_steps(
            solution.theta1_deg,
            config.steps_per_rev,
            config.microstep,
            GEAR_RATIO,
        ),
        deg_to_steps(
            solution.theta2_deg,
            config.steps_per_rev,
            config.microstep,
            GEAR_RATIO,
        ),
        deg_to_steps(
            solution.theta_base_deg,
            config.steps_per_rev,
            config.microstep,
            GEAR_RATIO,
        ),
    ];

    print_plan(&solution, &steps);

    #[cfg(all(feature = "hardware", target_os = "linux"))]
    run_hardware(&config, steps, overhead_us)?;

    #[cfg(not(all(feature = "hardware", target_os = "linux")))]
    {
        let _ = overhead_us;
        println!(
            "\nSimulation only: this binary was built without hardware support, so no GPIO signals were sent."
        );
        println!(
            "Build on a Raspberry Pi with 'cargo build --release --features hardware' for motor control."
        );
    }

    Ok(())
}

fn execute_position(config: MotionConfig) -> Result<(), Box<dyn std::error::Error>> {
    let solution = ik_angles_3d_deg(
        config.x_mm,
        config.y_mm,
        config.z_mm,
        config.l1_mm,
        config.l2_mm,
    )
    .map_err(|e| format!("IK error: {e}"))?;
    execute_solution(&config, solution)
}

fn run_position_loop(api: bool) -> Result<(), Box<dyn std::error::Error>> {
    if api {
        println!("API mode ready. Send one position command per line.");
    } else {
        println!("Shell mode. Enter one position command per line, or press Ctrl+D to exit.");
        println!("Format (X = radius, Y = base angle in degrees, Z = height):");
        println!(
            "radius_mm base_angle_deg height_mm l1_mm l2_mm steps_per_rev microstep ccw_positive"
        );
        println!("Example: 100 0 50 200 200 200 16 1");
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
                Err(error) => json!({"ok": false, "error": {"message": error}}),
            };
            println!("{}", response);
        } else {
            match config_from_line(&line).and_then(execute_position) {
                Ok(()) => println!("Command completed."),
                Err(error) => eprintln!("Command failed: {error}"),
            }
        }
    }
    Ok(())
}

fn run_raw_loop() -> Result<(), Box<dyn std::error::Error>> {
    println!("Raw mode. Enter: base_deg axis1_deg axis2_deg steps_per_rev microstep ccw_positive");
    println!("Press Ctrl+D to exit.");
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

fn read_http_request(stream: &mut TcpStream) -> Result<(String, String, String), String> {
    const HEADER_LIMIT: usize = 16 * 1024;
    let mut buffer = Vec::new();
    let header_end = loop {
        let mut chunk = [0u8; 1024];
        let count = stream.read(&mut chunk).map_err(|error| error.to_string())?;
        if count == 0 {
            return Err("connection closed before request headers".to_owned());
        }
        buffer.extend_from_slice(&chunk[..count]);
        if let Some(end) = buffer.windows(4).position(|window| window == b"\r\n\r\n") {
            break end + 4;
        }
        if buffer.len() > HEADER_LIMIT {
            return Err("request headers are too large".to_owned());
        }
    };

    let (method, target, content_length) = {
        let header_text =
            std::str::from_utf8(&buffer[..header_end]).map_err(|_| "invalid HTTP headers")?;
        let mut lines = header_text.split("\r\n");
        let request_line = lines
            .next()
            .ok_or_else(|| "missing HTTP request line".to_owned())?;
        let mut request_parts = request_line.split_whitespace();
        let method = request_parts
            .next()
            .ok_or_else(|| "missing HTTP method".to_owned())?;
        let target = request_parts
            .next()
            .ok_or_else(|| "missing HTTP target".to_owned())?;
        let version = request_parts
            .next()
            .ok_or_else(|| "missing HTTP version".to_owned())?;
        if version != "HTTP/1.1" && version != "HTTP/1.0" {
            return Err("unsupported HTTP version".to_owned());
        }

        let content_length = lines
            .filter_map(|line| line.split_once(':'))
            .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
            .map(|(_, value)| value.trim().parse::<usize>())
            .transpose()
            .map_err(|_| "invalid Content-Length".to_owned())?
            .unwrap_or(0);
        (method.to_owned(), target.to_owned(), content_length)
    };
    if content_length > 1024 * 1024 {
        return Err("request body is too large".to_owned());
    }

    while buffer.len() - header_end < content_length {
        let mut chunk = [0u8; 1024];
        let count = stream.read(&mut chunk).map_err(|error| error.to_string())?;
        if count == 0 {
            return Err("connection closed before request body".to_owned());
        }
        buffer.extend_from_slice(&chunk[..count]);
    }
    let body = std::str::from_utf8(&buffer[header_end..header_end + content_length])
        .map_err(|_| "request body is not UTF-8")?;
    Ok((method, target, body.to_owned()))
}

fn write_http_response(
    stream: &mut TcpStream,
    status: &str,
    body: &Value,
) -> Result<(), Box<dyn std::error::Error>> {
    let body = serde_json::to_string(body)?;
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )?;
    stream.flush()?;
    Ok(())
}

fn handle_api_connection(mut stream: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    match read_http_request(&mut stream) {
        Ok((method, target, body)) => match api_command_for_request(&method, &target, &body) {
            Ok(command) => {
                write_http_response(&mut stream, "200 OK", &api_command_response(command))?
            }
            Err(error) if error == "method not allowed" => write_http_response(
                &mut stream,
                "405 Method Not Allowed",
                &json!({"ok": false, "error": {"message": error}}),
            )?,
            Err(error) if error == "unknown API route" => write_http_response(
                &mut stream,
                "404 Not Found",
                &json!({"ok": false, "error": {"message": error}}),
            )?,
            Err(error) => write_http_response(
                &mut stream,
                "400 Bad Request",
                &json!({"ok": false, "error": {"message": error}}),
            )?,
        },
        Err(error) => write_http_response(
            &mut stream,
            "400 Bad Request",
            &json!({"ok": false, "error": {"message": error}}),
        )?,
    }
    Ok(())
}

fn run_api() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let address = env::var("RUSTCTL_API_ADDR").unwrap_or_else(|_| format!("127.0.0.1:{API_PORT}"));
    let listener = TcpListener::bind(&address)?;
    println!(
        "API listening on http://{address} hardware_enabled={} routes=/status,/help,/test,/args,/raw",
        hardware_enabled()
    );
    io::stdout().flush()?;
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => handle_api_connection(stream)?,
            Err(error) => eprintln!("API connection failed: {error}"),
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    get_mode(args)
}

fn get_mode(args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    match args.get(1).map(String::as_str) {
        Some("--cli") => execute_position(prompt_position()?),
        Some("--shell") => run_position_loop(false),
        Some("--api") => run_api(),
        Some("--raw") => run_raw_loop(),
        Some("--help") | None => {
            print_help(&args[0]);
            Ok(())
        }
        Some(mode) => Err(format!("Unknown mode '{mode}'. Use --help for usage.").into()),
    }
}

#[cfg(all(feature = "hardware", target_os = "linux"))]
fn run_hardware(
    config: &MotionConfig,
    steps: [i64; NUM_AXES],
    overhead_us: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let gpio = Gpio::new()?;
    let mut motors: [StepperMotor; NUM_AXES] = [
        StepperMotor::new(&gpio, PIN_AXIS1.0, PIN_AXIS1.1)?,
        StepperMotor::new(&gpio, PIN_AXIS2.0, PIN_AXIS2.1)?,
        StepperMotor::new(&gpio, PIN_BASE.0, PIN_BASE.1)?,
    ];
    let mut spare = StepperMotor::new(&gpio, PIN_ENDEFFECTOR.0, PIN_ENDEFFECTOR.1)?;
    spare.reset();

    for i in 0..NUM_AXES {
        motors[i].set_direction_ccw(direction_is_ccw(steps[i], config.ccw_positive));
    }

    let terminate = Arc::new(AtomicBool::new(false));
    {
        let t = Arc::clone(&terminate);
        ctrlc::set_handler(move || t.store(true, Ordering::SeqCst))?;
    }

    let stepped = run_motion(
        &mut motors,
        steps,
        config.pulse_t_us,
        overhead_us,
        &terminate,
    );

    for m in motors.iter_mut() {
        m.reset();
    }
    spare.reset();

    println!("\nExecution finished.");
    println!(
        "Processed steps -> Base: {} (target: {}), Axis 1: {} (target: {}), Axis 2: {} (target: {})",
        stepped[2], steps[2], stepped[0], steps[0], stepped[1], steps[1]
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    include!("tests.rs");
}
