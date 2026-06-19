use std::thread;
use std::time::Duration;
use std::io;

use rppal::gpio::Gpio;

fn main() {
    let gpio: Gpio = Gpio::new().expect("Failed to initialize GPIO access");
    
    println!("Enter Pin to pulse");

    let mut choice_str = String::new();

    io::stdin()
        .read_line(&mut choice_str)
        .expect("Failed to read num");

    let choice_int:u8 = choice_str.trim().parse().expect("type valid int");
    
    let mut pin: rppal::gpio::OutputPin = gpio.get(choice_int)
        .expect("Failed to claim pin 17")
        .into_output();

    loop {
        pin.set_high();
        thread::sleep(Duration::from_micros(500-55));
        pin.set_low();
        thread::sleep(Duration::from_micros(500-55));
    }
}