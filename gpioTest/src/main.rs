use std::thread;
use std::time::Duration;

use rppal::gpio::Gpio;

fn main() {
    // If Gpio initialization fails, the program crashes with this message
    let gpio: Gpio = Gpio::new().expect("Failed to initialize GPIO access");
    
    let mut pin: rppal::gpio::OutputPin = gpio.get(17)
        .expect("Failed to claim pin 17")
        .into_output();

    loop {
        pin.set_high();
        thread::sleep(Duration::from_micros(500));
        pin.set_low();
        thread::sleep(Duration::from_micros(500));
    }
}