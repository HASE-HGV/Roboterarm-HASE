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

fn prompt<T: std::str::FromStr>(msg: &str) -> T {
    loop {
        print!("{msg}");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        if let Ok(val) = input.trim().parse() {
            return val;
        }
        println!("Ungültige Eingabe. Bitte erneut versuchen.");
    }
}

fn prompt_config() -> MotionConfig {
    println!("Interaktiver Modus");
    let total_time_us = prompt("Gesamtzeit pro Schrittperiode (µs) [z.B. 1000]: ");
    let pulse_t_us = prompt("Puls-Dauer (µs) [z.B. 200]: ");
    let x_mm = prompt("Ziel X (mm): ");
    let y_mm = prompt("Ziel Y (mm) [Basis-Rotation]: ");
    let z_mm = prompt("Ziel Z (mm): ");
    let l1_mm = prompt("Länge Arm 1 (mm): ");
    let l2_mm = prompt("Länge Arm 2 (mm): ");
    let steps_per_rev = prompt("Schritte pro Umdrehung (Motor) [z.B. 200]: ");
    let microstep = prompt("Mikroschritt-Auflösung (Driver) [z.B. 1, 2, 16]: ");
    let ccw: u8 = prompt("CCW positiv? (1 = Ja, 0 = Nein): ");
    MotionConfig {
        total_time_us,
        pulse_t_us,
        x_mm,
        y_mm,
        z_mm,
        l1_mm,
        l2_mm,
        steps_per_rev,
        microstep,
        ccw_positive: ccw != 0,
    }
}

fn build_config(args: &[String]) -> Result<MotionConfig, Box<dyn std::error::Error>> {
    if args.len() == 1 {
        Ok(prompt_config())
    } else if args.len() < 11 {
        Err(format!(
            "Usage: {} <total_time_micros> <pulse_t_micros> <x_mm> <y_mm> <z_mm> \
             <l1_mm> <l2_mm> <steps_per_rev> <microstep> <ccw_positive(0/1)>",
            args[0]
        )
            .into())
    } else {
        Ok(MotionConfig {
            total_time_us: args[1].parse()?,
            pulse_t_us: args[2].parse()?,
            x_mm: args[3].parse()?,
            y_mm: args[4].parse()?,
            z_mm: args[5].parse()?,
            l1_mm: args[6].parse()?,
            l2_mm: args[7].parse()?,
            steps_per_rev: args[8].parse()?,
            microstep: args[9].parse()?,
            ccw_positive: args[10].parse::<u8>()? != 0,
        })
    }
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

    let theta_base = y_mm.atan2(x_mm);
    let r = (x_mm * x_mm + y_mm * y_mm).sqrt();
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

struct MultiAxisPlanner<const N: usize> {
    counts: [i64; N],
    accum: [i64; N],
    max_steps: i64,
    remaining: i64,
}

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
    println!("\n=== Kinematik Berechnungen ===");
    println!(
        "Theta Base: {:.3}°, Theta1: {:.3}°, Theta2: {:.3}°, z_eff: {:.3} mm",
        s.theta_base_deg, s.theta1_deg, s.theta2_deg, s.z_eff_mm
    );
    println!(
        "Zielschritte (16:1 Getriebe): Base: {}, Axis1: {}, Axis2: {}",
        steps[2], steps[0], steps[1]
    );
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let config = build_config(&args)?;

    let overhead_us = match overhead_sleep_us(config.total_time_us, config.pulse_t_us) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };

    let solution = ik_angles_3d_deg(
        config.x_mm,
        config.y_mm,
        config.z_mm,
        config.l1_mm,
        config.l2_mm,
    )
        .map_err(|e| format!("IK error: {e}"))?;

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
        println!("\n(ohne `hardware`-Feature gebaut — Motoren werden nicht angesteuert.)");
    }

    Ok(())
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

    println!("\nAusführung beendet.");
    println!(
        "Verarbeitete Schritte -> Base: {} (Soll: {}), Axis1: {} (Soll: {}), Axis2: {} (Soll: {})",
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
        let s = ik_angles_3d_deg(0.0, 100.0, 100.0, 100.0, 100.0).unwrap();
        assert!(approx(s.theta_base_deg, 90.0));
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
        for &(x, y, z) in &[(150.0, 30.0, 40.0), (80.0, -60.0, 20.0), (0.0, 0.0, 50.0)] {
            let s = ik_angles_3d_deg(x, y, z, l1, l2).unwrap();
            let (r_eff, z_eff) = forward_r_z_mm(s.theta1_deg, s.theta2_deg, l1, l2);
            let r = (x * x + y * y).sqrt();
            assert!(approx(r_eff, r), "r mismatch: {r_eff} vs {r}");
            assert!(approx(z_eff, z), "z mismatch: {z_eff} vs {z}");
            assert!(approx(s.theta_base_deg, y.atan2(x).to_degrees()));
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
