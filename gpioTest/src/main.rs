use std::thread;
use std::time::Duration;
use std::io;

use rppal::gpio::Gpio;

fn main() {
    let gpio: Gpio = Gpio::new().expect("Failed to initialize GPIO access");
    
    println!("Enter Pin to pulse:");
    let mut choice_str = String::new();
    io::stdin()
        .read_line(&mut choice_str)
        .expect("Failed to read num");
    let choice_int: u8 = choice_str.trim().parse().expect("Type valid int");
    
    println!("Enter delay time in microseconds:");
    let mut delay_str = String::new();
    io::stdin()
        .read_line(&mut delay_str)
        .expect("Failed to read delay");
    let delay_int: u64 = delay_str.trim().parse().expect("Type valid number");

    let final_delay = if delay_int > 55 { delay_int - 55 } else { 0 };

    let mut pin: rppal::gpio::OutputPin = gpio.get(choice_int)
        .expect("Failed to claim pin")
        .into_output();

    println!("Pulsing Pin {} with a delay of {} µs...", choice_int, final_delay);

    loop {
        pin.set_high();
        thread::sleep(Duration::from_micros(final_delay));
        pin.set_low();
        thread::sleep(Duration::from_micros(final_delay));
    }
}
