use std::{env, sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}}, time::Duration, thread};
use rppal::gpio::{Gpio, OutputPin};
use ctrlc;

struct StepperMotor {
    step_pin: OutputPin,
    dir_pin: OutputPin,
}

impl StepperMotor {
    fn set_direction(&mut self, ccw: bool) {
        if ccw { let _ = self.dir_pin.set_high(); } else { let _ = self.dir_pin.set_low(); }
    }

    fn set_step(&mut self, high: bool) {
        if high { let _ = self.step_pin.set_high(); } else { let _ = self.step_pin.set_low(); }
    }

    fn reset(&mut self) {
        let _ = self.step_pin.set_low();
        let _ = self.dir_pin.set_low();
    }
}

fn ik_angles_deg(x_mm: f64, z_mm: f64, l1_mm: f64, l2_mm: f64) -> Result<(f64,f64,f64), &'static str> {
    let r = (x_mm*x_mm + z_mm*z_mm).sqrt();
    if r > l1_mm + l2_mm { return Err("Out of workspace"); }
    let alpha = z_mm.atan2(x_mm);
    let cos_theta2 = ((r*r - l1_mm*l1_mm - l2_mm*l2_mm) / (2.0 * l1_mm * l2_mm)).clamp(-1.0, 1.0);
    let theta2 = cos_theta2.acos();
    let theta1 = alpha - ( (l2_mm*theta2.sin()).atan2( l1_mm + l2_mm*theta2.cos() ) );
    let z_eff = l1_mm*theta1.sin() + l2_mm*(theta1 + theta2).sin();
    Ok((theta1.to_degrees(), theta2.to_degrees(), z_eff))
}

fn deg_to_steps(angle_deg: f64, steps_per_rev: u64, microstep: u64) -> i64 {
    let steps_per_deg = (steps_per_rev * microstep) as f64 / 360.0;
    (angle_deg * steps_per_deg).round() as i64
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 10 {
        eprintln!("Usage: {} <total_time_micros> <pulse_t_micros> <x_mm> <z_mm> <l1_mm> <l2_mm> <steps_per_rev> <microstep> <ccw_positive(0/1)>", args[0]);
        std::process::exit(1);
    }

    let total_time: u64 = args[1].parse().map_err(|_| "total_time parse")?;
    let pulse_t_us: u64 = args[2].parse().map_err(|_| "pulse_t parse")?;
    let x_mm: f64 = args[3].parse()?;
    let z_mm: f64 = args[4].parse()?;
    let l1_mm: f64 = args[5].parse()?;
    let l2_mm: f64 = args[6].parse()?;
    let steps_per_rev: u64 = args[7].parse()?;
    let microstep: u64 = args[8].parse()?;
    let ccw_positive: bool = args[9].parse::<u8>()? != 0;

    if total_time <= 83 {
        eprintln!("Error: total_time must be >83 due to hardware overhead");
        std::process::exit(1);
    }

    let (th1_deg, th2_deg, z_eff) = ik_angles_deg(x_mm, z_mm, l1_mm, l2_mm).map_err(|e| format!("IK error: {}", e))?;
    let steps1 = deg_to_steps(th1_deg, steps_per_rev, microstep);
    let steps2 = deg_to_steps(th2_deg, steps_per_rev, microstep);

    let gpio = Gpio::new()?;

    let m1 = StepperMotor { step_pin: gpio.get(17)?.into_output(), dir_pin: gpio.get(27)?.into_output() };
    let m2 = StepperMotor { step_pin: gpio.get(22)?.into_output(), dir_pin: gpio.get(23)?.into_output() };
    let m3 = StepperMotor { step_pin: gpio.get(24)?.into_output(), dir_pin: gpio.get(25)?.into_output() };
    let m4 = StepperMotor { step_pin: gpio.get(5)?.into_output(), dir_pin: gpio.get(6)?.into_output() };

    let mut motors = vec![m1, m2, m3, m4];

    motors[0].set_direction((steps1 > 0) == ccw_positive);
    motors[1].set_direction((steps2 > 0) == ccw_positive);

    let shared_motors = Arc::new(Mutex::new(Some(motors)));
    let terminate = Arc::new(AtomicBool::new(false));
    {
        let s = Arc::clone(&shared_motors);
        let t = Arc::clone(&terminate);
        ctrlc::set_handler(move || {
            t.store(true, Ordering::SeqCst);
            if let Ok(mut guard) = s.lock() {
                if let Some(ref mut list) = *guard {
                    for m in list.iter_mut() { m.reset(); }
                }
            }
        })?;
    }

    println!("Theta1: {:.3}°, Theta2: {:.3}°, z_eff: {:.3} mm", th1_deg, th2_deg, z_eff);
    println!("Target steps: {}, {}", steps1, steps2);
    println!("Starting multi-axis synchronized movement...");

    let overhead_sleep = total_time.saturating_sub(83);
    
    let total_steps1 = steps1.abs();
    let total_steps2 = steps2.abs();
    let max_steps = total_steps1.max(total_steps2);

    let mut stepped1 = 0;
    let mut stepped2 = 0;
    let mut accum1 = 0;
    let mut accum2 = 0;

    for _ in 0..max_steps {
        if terminate.load(Ordering::SeqCst) { break; }

        let mut pulse_m1 = false;
        let mut pulse_m2 = false;

        accum1 += total_steps1;
        if accum1 >= max_steps {
            pulse_m1 = true;
            accum1 -= max_steps;
        }

        accum2 += total_steps2;
        if accum2 >= max_steps {
            pulse_m2 = true;
            accum2 -= max_steps;
        }

        if let Ok(mut guard) = shared_motors.lock() {
            if let Some(ref mut list) = *guard {
                if pulse_m1 { list[0].set_step(true); }
                if pulse_m2 { list[1].set_step(true); }
            }
        }

        thread::sleep(Duration::from_micros(pulse_t_us));

        if let Ok(mut guard) = shared_motors.lock() {
            if let Some(ref mut list) = *guard {
                if pulse_m1 { list[0].set_step(false); stepped1 += 1; }
                if pulse_m2 { list[1].set_step(false); stepped2 += 1; }
            }
        }

        thread::sleep(Duration::from_micros(pulse_t_us));
        thread::sleep(Duration::from_micros(overhead_sleep));
    }

    // Finale Bereinigung
    if let Ok(mut guard) = shared_motors.lock() {
        if let Some(mut list) = guard.take() {
            for motor in list.iter_mut() { motor.reset(); }
        }
    }

    println!("Done. Processed steps: {}, (Target: {}, {})", stepped1, steps1, steps2);
    Ok(())
}