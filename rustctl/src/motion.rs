use std::error::Error;

use crate::config::{GEAR_RATIO, MotionConfig, NUM_AXES};
use crate::kinematics::{ArmSolution, deg_to_steps, ik_angles_3d_deg, overhead_sleep_us};
use crate::pretty;

pub(crate) fn execute_position(config: MotionConfig) -> Result<(), Box<dyn Error>> {
    let solution = ik_angles_3d_deg(
        config.x_mm,
        config.y_mm,
        config.z_mm,
        config.l1_mm,
        config.l2_mm,
    )
    .map_err(|error| format!("IK error: {error}"))?;
    execute_solution(&config, solution)
}

pub(crate) fn execute_solution(
    config: &MotionConfig,
    solution: ArmSolution,
) -> Result<(), Box<dyn Error>> {
    let overhead_us = overhead_sleep_us(config.total_time_us, config.pulse_t_us)?;
    let steps = [
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
    run_hardware(config, steps, overhead_us)?;

    #[cfg(not(all(feature = "hardware", target_os = "linux")))]
    {
        let _ = overhead_us;
        println!(
            "{}",
            pretty::warning(
                "Simulation only: this binary was built without hardware support, so no GPIO signals were sent."
            )
        );
        println!(
            "{}",
            pretty::info(
                "Build on a Raspberry Pi with 'cargo build --release --features hardware' for motor control."
            )
        );
    }
    Ok(())
}

fn print_plan(solution: &ArmSolution, steps: &[i64; NUM_AXES]) {
    println!("{}", pretty::title("Kinematics"));
    println!(
        "Base angle: {:.3}°, Axis 1: {:.3}°, Axis 2: {:.3}°, effective Z: {:.3} mm",
        solution.theta_base_deg, solution.theta1_deg, solution.theta2_deg, solution.z_eff_mm
    );
    println!(
        "{}",
        pretty::info(&format!(
            "Target steps (16:1 gearbox): Base: {}, Axis 1: {}, Axis 2: {}",
            steps[2], steps[0], steps[1]
        ))
    );
}

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
use crate::bresenham::MultiAxisPlanner;

#[cfg(all(feature = "hardware", target_os = "linux"))]
use rppal::gpio::{Gpio, OutputPin};

#[cfg(all(feature = "hardware", target_os = "linux"))]
const HARDWARE_OVERHEAD_US: u64 = 83;
#[cfg(all(feature = "hardware", target_os = "linux"))]
const PIN_AXIS1: (u8, u8) = (17, 27);
#[cfg(all(feature = "hardware", target_os = "linux"))]
const PIN_AXIS2: (u8, u8) = (22, 23);
#[cfg(all(feature = "hardware", target_os = "linux"))]
const PIN_BASE: (u8, u8) = (24, 25);
#[cfg(all(feature = "hardware", target_os = "linux"))]
const PIN_ENDEFFECTOR: (u8, u8) = (5, 6);

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
fn run_hardware(
    config: &MotionConfig,
    steps: [i64; NUM_AXES],
    overhead_us: u64,
) -> Result<(), Box<dyn Error>> {
    let gpio = Gpio::new()?;
    let mut motors = [
        StepperMotor::new(&gpio, PIN_AXIS1.0, PIN_AXIS1.1)?,
        StepperMotor::new(&gpio, PIN_AXIS2.0, PIN_AXIS2.1)?,
        StepperMotor::new(&gpio, PIN_BASE.0, PIN_BASE.1)?,
    ];
    let mut spare = StepperMotor::new(&gpio, PIN_ENDEFFECTOR.0, PIN_ENDEFFECTOR.1)?;
    spare.reset();
    for (motor, step) in motors.iter_mut().zip(steps) {
        motor.set_direction_ccw(crate::kinematics::direction_is_ccw(
            step,
            config.ccw_positive,
        ));
    }
    let terminate = Arc::new(AtomicBool::new(false));
    let signal = Arc::clone(&terminate);
    ctrlc::set_handler(move || signal.store(true, Ordering::SeqCst))?;
    let stepped = run_motion(
        &mut motors,
        steps,
        config.pulse_t_us,
        overhead_us,
        &terminate,
    );
    for motor in &mut motors {
        motor.reset();
    }
    spare.reset();
    println!("{}", pretty::success("Execution finished."));
    println!(
        "Processed steps -> Base: {} (target: {}), Axis 1: {} (target: {}), Axis 2: {} (target: {})",
        stepped[2], steps[2], stepped[0], steps[0], stepped[1], steps[1]
    );
    Ok(())
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
    let mut stepped = [0; NUM_AXES];
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
        debug_assert_eq!(stepped, expected);
    }
    stepped
}
