use std::{
    env,
    io::{self, Write},
};

#[cfg(feature = "hardware")]
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

#[cfg(feature = "hardware")]
use rppal::gpio::{Gpio, OutputPin};

const GEAR_RATIO: f64 = 16.0;
const TOTAL_TIME_US: u64 = 1083;
const PULSE_T_US: u64 = 500;
const HARDWARE_OVERHEAD_US: u64 = 83;
const NUM_AXES: usize = 3;

#[cfg(feature = "hardware")]
const PIN_AXIS1: (u8, u8) = (17, 27);
#[cfg(feature = "hardware")]
const PIN_AXIS2: (u8, u8) = (22, 23);
#[cfg(feature = "hardware")]
const PIN_BASE: (u8, u8) = (24, 25);
#[cfg(feature = "hardware")]
const PIN_SPARE: (u8, u8) = (5, 6);

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

#[derive(Debug, Clone, Copy)]
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
    println!("Usage: {program} --cli | --args | --raw | --api | --help");
    println!("\nModes:");
    println!("  --cli   Prompt for one XYZ position and execute it.");
    println!("  --args  Repeatedly read radius/angle/height commands from an interactive input loop.");
    println!("  --raw   Repeatedly read raw angles: base_deg axis1_deg axis2_deg steps_per_rev microstep ccw_positive.");
    println!("  --api   api server (to be added)");
    println!("  --help  Show this guide.");
    println!("\nPosition command format:");
    println!("  radius_mm base_angle_deg height_mm l1_mm l2_mm steps_per_rev microstep ccw_positive");
    println!("Timing is fixed: total period = {TOTAL_TIME_US} µs, pulse width = {PULSE_T_US} µs.");
    println!("\nPC testing:");
    println!("  cargo run -- --cli");
    println!("  cargo run -- --args");
    println!("  cargo test");
    println!("\nRaspberry Pi hardware:");
    println!("  cargo build --release --features hardware");
    println!("  sudo ./target/release/rustctl --args");
    println!("Commands are processed until EOF or Ctrl+C. Without the hardware feature, no GPIO is driven.");
}

fn raw_command(line: &str) -> Result<(MotionConfig, ArmSolution), Box<dyn std::error::Error>> {
    let values: Vec<&str> = line.split_whitespace().collect();
    if values.len() != 6 {
        return Err("Expected: base_deg axis1_deg axis2_deg steps_per_rev microstep ccw_positive".into());
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

#[derive(Debug, Clone, Copy, PartialEq)]
struct ArmSolution {
    theta_base_deg: f64,
    theta1_deg: f64,
    theta2_deg: f64,
    z_eff_mm: f64,
}

fn forward_r_z_mm(theta1_deg: f64, theta2_deg: f64, l1_mm: f64, l2_mm: f64) -> (f64, f64) {
    let t1 = theta1_deg.to_radians();
    let t12 = (theta1_deg + theta2_deg).to_radians();
    let r = l1_mm * t1.cos() + l2_mm * t12.cos();
    let z = l1_mm * t1.sin() + l2_mm * t12.sin();
    (r, z)
}

fn ik_angles_3d_deg(
    x_mm: f64,
    y_mm: f64,
    z_mm: f64,
    l1_mm: f64,
    l2_mm: f64,
) -> Result<ArmSolution, &'static str> {
    if l1_mm <= 0.0 || l2_mm <= 0.0 {
        return Err("Link lengths must be positive");
    }
    if x_mm < 0.0 {
        return Err("Radius X must be non-negative");
    }

    let theta_base = y_mm.to_radians();
    let r = x_mm;
    let r_space = (r * r + z_mm * z_mm).sqrt();
    if r_space > l1_mm + l2_mm {
        return Err("Out of workspace");
    }

    let alpha = z_mm.atan2(r);
    let cos_theta2 = ((r_space * r_space - l1_mm * l1_mm - l2_mm * l2_mm)
        / (2.0 * l1_mm * l2_mm))
        .clamp(-1.0, 1.0);
    let theta2 = cos_theta2.acos();
    let theta1 = alpha - (l2_mm * theta2.sin()).atan2(l1_mm + l2_mm * theta2.cos());

    let (r_eff, z_eff) = forward_r_z_mm(theta1.to_degrees(), theta2.to_degrees(), l1_mm, l2_mm);

    debug_invariant!(
        (r_eff - r).abs() < 1e-6 && (z_eff - z_mm).abs() < 1e-6,
        r,
        z_mm,
        r_eff,
        z_eff
    );

    Ok(ArmSolution {
        theta_base_deg: theta_base.to_degrees(),
        theta1_deg: theta1.to_degrees(),
        theta2_deg: theta2.to_degrees(),
        z_eff_mm: z_eff,
    })
}

fn deg_to_steps(angle_deg: f64, steps_per_rev: u64, microstep: u64, gear_ratio: f64) -> i64 {
    let steps_per_deg = (steps_per_rev * microstep) as f64 / 360.0;
    (angle_deg * steps_per_deg * gear_ratio).round() as i64
}

#[cfg(any(feature = "hardware", test))]
fn direction_is_ccw(steps: i64, ccw_positive: bool) -> bool {
    (steps > 0) == ccw_positive
}

fn minimum_period_us(pulse_t_us: u64) -> u64 {
    2 * pulse_t_us + HARDWARE_OVERHEAD_US
}

fn overhead_sleep_us(total_time_us: u64, pulse_t_us: u64) -> Result<u64, String> {
    let min = minimum_period_us(pulse_t_us);
    if total_time_us < min {
        return Err(format!(
            "total_time ({total_time_us}) must be >= {min} µs to accommodate \
             pulse widths and hardware overhead"
        ));
    }
    Ok(total_time_us - min)
}

#[cfg(any(feature = "hardware", test))]
struct MultiAxisPlanner<const N: usize> {
    counts: [i64; N],
    accum: [i64; N],
    max_steps: i64,
    remaining: i64,
}

#[cfg(any(feature = "hardware", test))]
impl<const N: usize> MultiAxisPlanner<N> {
    fn new(steps: [i64; N]) -> Self {
        let counts: [i64; N] = std::array::from_fn(|i| steps[i].abs());
        let max_steps = counts.iter().copied().max().unwrap_or(0);
        Self {
            counts,
            accum: [0; N],
            max_steps,
            remaining: max_steps,
        }
    }
}

#[cfg(any(feature = "hardware", test))]
impl<const N: usize> Iterator for MultiAxisPlanner<N> {
    type Item = [bool; N];

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining <= 0 {
            return None;
        }
        self.remaining -= 1;

        let mut pulses = [false; N];
        for i in 0..N {
            self.accum[i] += self.counts[i];
            if self.accum[i] >= self.max_steps {
                pulses[i] = true;
                self.accum[i] -= self.max_steps;
            }
            debug_assert!(self.accum[i] >= 0 && self.accum[i] < self.max_steps.max(1));
        }
        Some(pulses)
    }
}

#[cfg(feature = "hardware")]
struct StepperMotor {
    step_pin: OutputPin,
    dir_pin: OutputPin,
}

#[cfg(feature = "hardware")]
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

#[cfg(feature = "hardware")]
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
    #[cfg(not(feature = "hardware"))]
    let _ = config.ccw_positive;

    let overhead_us = match overhead_sleep_us(config.total_time_us, config.pulse_t_us) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };

    let steps: [i64; NUM_AXES] = [
        deg_to_steps(solution.theta1_deg, config.steps_per_rev, config.microstep, GEAR_RATIO),
        deg_to_steps(solution.theta2_deg, config.steps_per_rev, config.microstep, GEAR_RATIO),
        deg_to_steps(solution.theta_base_deg, config.steps_per_rev, config.microstep, GEAR_RATIO),
    ];

    print_plan(&solution, &steps);

    #[cfg(feature = "hardware")]
    run_hardware(&config, steps, overhead_us)?;

    #[cfg(not(feature = "hardware"))]
    {
        let _ = overhead_us;
        println!("\nSimulation only: this binary was built without hardware support, so no GPIO signals were sent.");
        println!("Build with 'cargo build --release --features hardware' on a Raspberry Pi for motor control.");
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
        println!("Args mode. Enter one position command per line, or press Ctrl+D to exit.");
        println!("Format (X = radius, Y = base angle in degrees, Z = height):");
        println!("radius_mm base_angle_deg height_mm l1_mm l2_mm steps_per_rev microstep ccw_positive");
        println!("Example: 100 0 50 200 200 200 16 1");
    }
    let stdin = io::stdin();
    loop {
        if !api {
            print!("args> ");
            io::stdout().flush()?;
        }
        let mut line = String::new();
        if stdin.read_line(&mut line)? == 0 {
            break;
        }
        if line.trim().is_empty() {
            continue;
        }
        match config_from_line(&line).and_then(execute_position) {
            Ok(()) => println!("Command completed."),
            Err(error) => eprintln!("Command failed: {error}"),
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
        match raw_command(&line).and_then(|(config, solution)| execute_solution(&config, solution)) {
            Ok(()) => println!("Command completed."),
            Err(error) => eprintln!("Command failed: {error}"),
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("--cli") => execute_position(prompt_position()?),
        Some("--args") => run_position_loop(false),
        Some("--api") => {
            println!("not yet implemented");
            Ok(())
        }
        Some("--raw") => run_raw_loop(),
        Some("--help") | None => {
            print_help(&args[0]);
            Ok(())
        }
        Some(mode) => Err(format!("Unknown mode '{mode}'. Use --help for usage.").into()),
    }
}

#[cfg(feature = "hardware")]
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
    let mut spare = StepperMotor::new(&gpio, PIN_SPARE.0, PIN_SPARE.1)?;
    spare.reset();

    for i in 0..NUM_AXES {
        motors[i].set_direction_ccw(direction_is_ccw(steps[i], config.ccw_positive));
    }

    let terminate = Arc::new(AtomicBool::new(false));
    {
        let t = Arc::clone(&terminate);
        ctrlc::set_handler(move || t.store(true, Ordering::SeqCst))?;
    }

    let stepped = run_motion(&mut motors, steps, config.pulse_t_us, overhead_us, &terminate);

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
    use super::*;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-6
    }

    #[test]
    fn deg_to_steps_full_revolution() {
        assert_eq!(deg_to_steps(360.0, 200, 1, GEAR_RATIO), 3200);
    }

    #[test]
    fn deg_to_steps_quarter_and_microstep() {
        assert_eq!(deg_to_steps(90.0, 200, 1, GEAR_RATIO), 800);
        assert_eq!(deg_to_steps(90.0, 200, 16, GEAR_RATIO), 12800);
    }

    #[test]
    fn deg_to_steps_sign_and_zero() {
        assert_eq!(deg_to_steps(-90.0, 200, 1, GEAR_RATIO), -800);
        assert_eq!(deg_to_steps(0.0, 200, 16, GEAR_RATIO), 0);
    }

    #[test]
    fn deg_to_steps_rounds_to_nearest() {
        assert_eq!(deg_to_steps(1.0, 200, 1, GEAR_RATIO), 9);
    }

    #[test]
    fn direction_logic_all_quadrants() {
        assert_eq!(direction_is_ccw(5, true), true);
        assert_eq!(direction_is_ccw(-5, true), false);
        assert_eq!(direction_is_ccw(5, false), false);
        assert_eq!(direction_is_ccw(-5, false), true);
    }

    #[test]
    fn overhead_normal() {
        assert_eq!(overhead_sleep_us(1000, 200).unwrap(), 517);
    }

    #[test]
    fn overhead_exact_minimum_is_zero() {
        assert_eq!(overhead_sleep_us(483, 200).unwrap(), 0);
    }

    #[test]
    fn overhead_below_minimum_errors() {
        assert!(overhead_sleep_us(482, 200).is_err());
    }

    #[test]
    fn ik_fully_extended_along_x() {
        let s = ik_angles_3d_deg(200.0, 0.0, 0.0, 100.0, 100.0).unwrap();
        assert!(approx(s.theta_base_deg, 0.0));
        assert!(approx(s.theta1_deg, 0.0));
        assert!(approx(s.theta2_deg, 0.0));
        assert!(approx(s.z_eff_mm, 0.0));
    }

    #[test]
    fn ik_right_angle_reach() {
        let s = ik_angles_3d_deg(100.0, 0.0, 100.0, 100.0, 100.0).unwrap();
        assert!(approx(s.theta1_deg, 0.0));
        assert!(approx(s.theta2_deg, 90.0));
        assert!(approx(s.z_eff_mm, 100.0));
    }

    #[test]
    fn ik_base_rotation_90() {
        let s = ik_angles_3d_deg(100.0, 90.0, 100.0, 100.0, 100.0).unwrap();
        assert!(approx(s.theta_base_deg, 90.0));
    }

    #[test]
    fn ik_negative_radius_rejected() {
        assert_eq!(
            ik_angles_3d_deg(-1.0, 0.0, 0.0, 100.0, 100.0),
            Err("Radius X must be non-negative")
        );
    }

    #[test]
    fn ik_out_of_workspace() {
        assert_eq!(
            ik_angles_3d_deg(300.0, 0.0, 0.0, 100.0, 100.0),
            Err("Out of workspace")
        );
    }

    #[test]
    fn ik_zero_link_rejected() {
        assert!(ik_angles_3d_deg(50.0, 0.0, 0.0, 0.0, 100.0).is_err());
        assert!(ik_angles_3d_deg(50.0, 0.0, 0.0, 100.0, 0.0).is_err());
    }

    #[test]
    fn ik_forward_roundtrip() {
        let (l1, l2) = (120.0, 90.0);
        for &(radius, base_angle, height) in &[(150.0, 30.0, 40.0), (80.0, -60.0, 20.0), (0.0, 90.0, 50.0)] {
            let s = ik_angles_3d_deg(radius, base_angle, height, l1, l2).unwrap();
            let (r_eff, z_eff) = forward_r_z_mm(s.theta1_deg, s.theta2_deg, l1, l2);
            assert!(approx(r_eff, radius), "radius mismatch: {r_eff} vs {radius}");
            assert!(approx(z_eff, height), "height mismatch: {z_eff} vs {height}");
            assert!(approx(s.theta_base_deg, base_angle));
        }
    }

    #[test]
    fn planner_emits_exact_counts() {
        let steps = [30i64, 12, 7];
        let mut totals = [0i64; 3];
        let mut ticks = 0;
        for p in MultiAxisPlanner::new(steps) {
            for i in 0..3 {
                if p[i] {
                    totals[i] += 1;
                }
            }
            ticks += 1;
        }
        assert_eq!(ticks, 30);
        assert_eq!(totals, [30, 12, 7]);
    }

    #[test]
    fn planner_dominant_axis_pulses_every_tick() {
        let plan: Vec<[bool; 3]> = MultiAxisPlanner::new([10i64, 0, 0]).collect();
        assert_eq!(plan.len(), 10);
        assert!(plan.iter().all(|t| t[0] && !t[1] && !t[2]));
    }

    #[test]
    fn planner_uses_step_magnitude() {
        let mut totals = [0i64; 3];
        for p in MultiAxisPlanner::new([-8i64, 4, -2]) {
            for i in 0..3 {
                if p[i] {
                    totals[i] += 1;
                }
            }
        }
        assert_eq!(totals, [8, 4, 2]);
    }

    #[test]
    fn planner_no_motion_yields_nothing() {
        assert_eq!(MultiAxisPlanner::new([0i64, 0, 0]).count(), 0);
    }

    #[test]
    fn planner_distributes_evenly() {
        let mut max_gap = 0;
        let mut gap = 0;
        for p in MultiAxisPlanner::new([10i64, 5, 0]) {
            if p[1] {
                max_gap = max_gap.max(gap);
                gap = 0;
            } else {
                gap += 1;
            }
        }
        assert!(max_gap <= 1, "pulses should be evenly spaced, gap={max_gap}");
    }
}
