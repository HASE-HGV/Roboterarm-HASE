use std::{env, sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}}, time::Duration, thread, io::{self, Write}};
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

// 3D Inverse Kinematics incorporating base rotation (y-axis)
fn ik_angles_3d_deg(x_mm: f64, y_mm: f64, z_mm: f64, l1_mm: f64, l2_mm: f64) -> Result<(f64, f64, f64, f64), &'static str> {
    // Calculate base rotation angle (Theta Base)
    let theta_base = y_mm.atan2(x_mm);
    
    // Project 3D coordinates onto the 2D planar arm extension (r = horizontal distance)
    let r = (x_mm * x_mm + y_mm * y_mm).sqrt();
    
    // Total distance from origin to TCP in 3D space
    let r_space = (r * r + z_mm * z_mm).sqrt();
    if r_space > l1_mm + l2_mm { return Err("Out of workspace"); }
    
    let alpha = z_mm.atan2(r);
    let cos_theta2 = ((r_space * r_space - l1_mm * l1_mm - l2_mm * l2_mm) / (2.0 * l1_mm * l2_mm)).clamp(-1.0, 1.0);
    let theta2 = cos_theta2.acos();
    let theta1 = alpha - ((l2_mm * theta2.sin()).atan2(l1_mm + l2_mm * theta2.cos()));
    let z_eff = l1_mm * theta1.sin() + l2_mm * (theta1 + theta2).sin();
    
    Ok((theta_base.to_degrees(), theta1.to_degrees(), theta2.to_degrees(), z_eff))
}

fn deg_to_steps(angle_deg: f64, steps_per_rev: u64, microstep: u64) -> i64 {
    const GEAR_RATIO: f64 = 16.0; 
    let steps_per_deg = (steps_per_rev * microstep) as f64 / 360.0;
    (angle_deg * steps_per_deg * GEAR_RATIO).round() as i64
}

// Helper function for safe interactive terminal inputs
fn prompt<T: std::str::FromStr>(msg: &str) -> T {
    loop {
        print!("{}", msg);
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        if let Ok(val) = input.trim().parse() {
            return val;
        }
        println!("Ungültige Eingabe. Bitte erneut versuchen.");
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    
    let total_time: u64;
    let pulse_t_us: u64;
    let x_mm: f64;
    let y_mm: f64;
    let z_mm: f64;
    let l1_mm: f64;
    let l2_mm: f64;
    let steps_per_rev: u64;
    let microstep: u64;
    let ccw_positive: bool;

    // Check if CLI arguments are missing -> Switch to interactive prompt mode
    if args.len() == 1 {
        println!("=== Interaktiver Modus (Keine CLI-Argumente übergeben) ===");
        total_time = prompt("Gesamtzeit pro Schrittperiode (µs) [z.B. 1000]: ");
        pulse_t_us = prompt("Puls-Dauer (µs) [z.B. 200]: ");
        x_mm = prompt("Ziel X (mm): ");
        y_mm = prompt("Ziel Y (mm) [Basis-Rotation]: ");
        z_mm = prompt("Ziel Z (mm): ");
        l1_mm = prompt("Länge Arm 1 (mm): ");
        l2_mm = prompt("Länge Arm 2 (mm): ");
        steps_per_rev = prompt("Schritte pro Umdrehung (Motor) [z.B. 200]: ");
        microstep = prompt("Mikroschritt-Auflösung (Driver) [z.B. 1, 2, 16]: ");
        let ccw: u8 = prompt("CCW positiv? (1 = Ja, 0 = Nein): ");
        ccw_positive = ccw != 0;
    } else if args.len() < 11 {
        eprintln!("Usage: {} <total_time_micros> <pulse_t_micros> <x_mm> <y_mm> <z_mm> <l1_mm> <l2_mm> <steps_per_rev> <microstep> <ccw_positive(0/1)>", args[0]);
        std::process::exit(1);
    } else {
        total_time = args[1].parse().map_err(|_| "total_time parse")?;
        pulse_t_us = args[2].parse().map_err(|_| "pulse_t parse")?;
        x_mm = args[3].parse()?;
        y_mm = args[4].parse()?;
        z_mm = args[5].parse()?;
        l1_mm = args[6].parse()?;
        l2_mm = args[7].parse()?;
        steps_per_rev = args[8].parse()?;
        microstep = args[9].parse()?;
        ccw_positive = args[10].parse::<u8>()? != 0;
    }

    let minimum_required_time = (2 * pulse_t_us) + 83;
    if total_time < minimum_required_time {
        eprintln!(
            "Error: total_time ({}) must be >= {} µs to accommodate pulse widths and hardware overhead", 
            total_time, minimum_required_time
        );
        std::process::exit(1);
    }

    // Process 3D Kinematics
    let (th_base_deg, th1_deg, th2_deg, z_eff) = ik_angles_3d_deg(x_mm, y_mm, z_mm, l1_mm, l2_mm)
        .map_err(|e| format!("IK error: {}", e))?;
        
    let steps_base = deg_to_steps(th_base_deg, steps_per_rev, microstep);
    let steps1 = deg_to_steps(th1_deg, steps_per_rev, microstep);
    let steps2 = deg_to_steps(th2_deg, steps_per_rev, microstep);

    let gpio = Gpio::new()?;

    // Motor mapping: m1 = Arm 1, m2 = Arm 2, m3 = Base Rotation, m4 = Auxiliary/Unused
    let m1 = StepperMotor { step_pin: gpio.get(17)?.into_output(), dir_pin: gpio.get(27)?.into_output() };
    let m2 = StepperMotor { step_pin: gpio.get(22)?.into_output(), dir_pin: gpio.get(23)?.into_output() };
    let m3 = StepperMotor { step_pin: gpio.get(24)?.into_output(), dir_pin: gpio.get(25)?.into_output() };
    let m4 = StepperMotor { step_pin: gpio.get(5)?.into_output(), dir_pin: gpio.get(6)?.into_output() };

    let mut motors = vec![m1, m2, m3, m4];

    // Set directions for all 3 operational axes
    motors[0].set_direction((steps1 > 0) == ccw_positive);
    motors[1].set_direction((steps2 > 0) == ccw_positive);
    motors[2].set_direction((steps_base > 0) == ccw_positive);

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

    println!("\n=== Kinematik Berechnungen ===");
    println!("Theta Base: {:.3}°, Theta1: {:.3}°, Theta2: {:.3}°, z_eff: {:.3} mm", th_base_deg, th1_deg, th2_deg, z_eff);
    println!("Zielschritte (16:1 Getriebe): Base: {}, Axis1: {}, Axis2: {}", steps_base, steps1, steps2);
    println!("Starte 3-Achsen synchronisierte Bewegung...");

    let overhead_sleep = total_time - minimum_required_time;
    
    let total_steps_b = steps_base.abs();
    let total_steps1 = steps1.abs();
    let total_steps2 = steps2.abs();
    
    // Find absolute maximum steps across all 3 axes for multi-axis synchronization
    let max_steps = total_steps_b.max(total_steps1).max(total_steps2);

    let mut stepped_b = 0;
    let mut stepped1 = 0;
    let mut stepped2 = 0;
    
    let mut accum_b = 0;
    let mut accum1 = 0;
    let mut shadow_accum2 = 0; // custom accumulator name to avoid collision

    for _ in 0..max_steps {
        if terminate.load(Ordering::SeqCst) { break; }

        let mut pulse_mb = false;
        let mut pulse_m1 = false;
        let mut pulse_m2 = false;

        accum_b += total_steps_b;
        if accum_b >= max_steps {
            pulse_mb = true;
            accum_b -= max_steps;
        }

        accum1 += total_steps1;
        if accum1 >= max_steps {
            pulse_m1 = true;
            accum1 -= max_steps;
        }

        shadow_accum2 += total_steps2;
        if shadow_accum2 >= max_steps {
            pulse_m2 = true;
            shadow_accum2 -= max_steps;
        }

        // Set Step-Pins HIGH
        if let Ok(mut guard) = shared_motors.lock() {
            if let Some(ref mut list) = *guard {
                if pulse_m1 { list[0].set_step(true); }
                if pulse_m2 { list[1].set_step(true); }
                if pulse_mb { list[2].set_step(true); }
            }
        }

        thread::sleep(Duration::from_micros(pulse_t_us));

        // Set Step-Pins LOW & increment counters
        if let Ok(mut guard) = shared_motors.lock() {
            if let Some(ref mut list) = *guard {
                if pulse_m1 { list[0].set_step(false); stepped1 += 1; }
                if pulse_m2 { list[1].set_step(false); stepped2 += 1; }
                if pulse_mb { list[2].set_step(false); stepped_b += 1; }
            }
        }

        thread::sleep(Duration::from_micros(pulse_t_us));
        if overhead_sleep > 0 {
            thread::sleep(Duration::from_micros(overhead_sleep));
        }
    }

    // Final Cleanup
    if let Ok(mut guard) = shared_motors.lock() {
        if let Some(mut list) = guard.take() {
            for motor in list.iter_mut() { motor.reset(); }
        }
    }

    println!("\nAusführung beendet.");
    println!("Verarbeitete Schritte -> Base: {} (Soll: {}), Axis1: {} (Soll: {}), Axis2: {} (Soll: {})", 
             stepped_b, steps_base, stepped1, steps1, stepped2, steps2);
    Ok(())
}