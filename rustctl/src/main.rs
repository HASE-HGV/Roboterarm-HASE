use std::{env, sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}}, time::Duration, thread};
use rppal::gpio::{Gpio, OutputPin};
use ctrlc;

struct StepperMotor {
    step_pin: OutputPin,
    dir_pin: OutputPin,
}

impl StepperMotor {
    fn set_direction(&mut self, ccw: bool) {
        if ccw {
            let _ = self.dir_pin.set_high();
        } else {
            let _ = self.dir_pin.set_low();
        }
    }

    fn step_toggle(&mut self) {
        self.step_pin.toggle();
    }

    fn reset(&mut self) {
        let _ = self.step_pin.set_low();
        let _ = self.dir_pin.set_low();
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <total_time_micros>", args[0]);
        std::process::exit(1);
    }

    let total_time: u64 = args[1].parse().map_err(|_| "Please provide a valid number for microsecond time")?;
    
    if total_time <= 55 {
        eprintln!("Error: Input time must be greater than 55 microseconds to account for hardware overhead.");
        std::process::exit(1);
    }
    
    let blink_interval = Duration::from_micros(total_time - 55);

    let gpio = Gpio::new()?;

    let m1 = StepperMotor {
        step_pin: gpio.get(17)?.into_output(),
        dir_pin: gpio.get(27)?.into_output(),
    };
    let m2 = StepperMotor {
        step_pin: gpio.get(22)?.into_output(),
        dir_pin: gpio.get(23)?.into_output(),
    };
    let m3 = StepperMotor {
        step_pin: gpio.get(24)?.into_output(),
        dir_pin: gpio.get(25)?.into_output(),
    };
    let m4 = StepperMotor {
        step_pin: gpio.get(5)?.into_output(),
        dir_pin: gpio.get(6)?.into_output(),
    };

    let mut motors = vec![m1, m2, m3, m4];

    motors[0].set_direction(false);
    motors[1].set_direction(true);
    motors[2].set_direction(false);
    motors[3].set_direction(true);

    let shared_motors = Arc::new(Mutex::new(Some(motors)));
    let terminate = Arc::new(AtomicBool::new(false));

    {
        let s = Arc::clone(&shared_motors);
        let t = Arc::clone(&terminate);
        
        ctrlc::set_handler(move || {
            t.store(true, Ordering::SeqCst);
            if let Ok(mut guard) = s.lock() {
                if let Some(ref mut list) = *guard {
                    for motor in list.iter_mut() {
                        motor.reset();
                    }
                }
            }
        })?;
    }

    println!("Running simultaneous motors with a sleep interval of {:?}", blink_interval);

    while !terminate.load(Ordering::SeqCst) {
        if let Ok(mut guard) = shared_motors.lock() {
            if let Some(ref mut list) = *guard {
                for motor in list.iter_mut() {
                    motor.step_toggle();
                }
            }
        }
        thread::sleep(blink_interval);
    }

    if let Ok(mut guard) = shared_motors.lock() {
        if let Some(mut list) = guard.take() {
            for motor in list.iter_mut() {
                motor.reset();
            }
        }
    }

    Ok(())
}