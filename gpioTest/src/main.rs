use std::thread;
use std::time::Duration;
use std::io;

use rppal::gpio::{Gpio, OutputPin};

struct MotorPins {
    pin1: OutputPin,
    pin2: OutputPin,
    pin3: OutputPin,
    pin4: OutputPin,
}

fn gpio_init() -> MotorPins {
    let gpio: Gpio = Gpio::new().expect("Failed to initialize GPIO access");

    MotorPins {
        pin1: gpio.get(17).expect("Failed to claim pin 17").into_output(),
        pin2: gpio.get(22).expect("Failed to claim pin 22").into_output(),
        pin3: gpio.get(24).expect("Failed to claim pin 24").into_output(),
        pin4: gpio.get(5).expect("Failed to claim pin 5").into_output(),
    }
}

fn m_drive_1m(mut pins: MotorPins) {
    let delay = delayHandler() - 70;
    loop {
        pins.pin1.set_high();
        thread::sleep(Duration::from_micros(delay));
        pins.pin1.set_low();
        thread::sleep(Duration::from_micros(delay));
    }
}

fn m_drive_2m(mut pins: MotorPins) {
    let delay = delayHandler();
    loop{
        pins.pin1.set_high();
        pins.pin2.set_high();
        thread::sleep(Duration::from_micros(delay));
        pins.pin1.set_low();
        pins.pin2.set_low();
        thread::sleep(Duration::from_micros(delay));
    }
    
}

fn m_drive_3m(mut pins: MotorPins) {
    let delay = delayHandler();
    loop {
        pins.pin1.set_high();
        pins.pin2.set_high();
        pins.pin3.set_high();
        thread::sleep(Duration::from_micros(delay));
        pins.pin1.set_low();
        pins.pin2.set_low();
        pins.pin3.set_low();
        thread::sleep(Duration::from_micros(delay));
    }
    
}

fn m_drive_4m(mut pins: MotorPins) {
    let delay = delayHandler();
    loop {
        pins.pin1.set_high();
        pins.pin2.set_high();
        pins.pin3.set_high();
        pins.pin4.set_high();
        thread::sleep(Duration::from_micros(delay));
        pins.pin1.set_low();
        pins.pin2.set_low();
        pins.pin3.set_low();
        pins.pin4.set_low();
        thread::sleep(Duration::from_micros(delay));
    }
    
}

fn multi_motor_handler(count: u8) {
    let pins = gpio_init(); 
    
    match count {
        1 => m_drive_1m(pins),
        2 => m_drive_2m(pins),
        3 => m_drive_3m(pins),
        4 => m_drive_4m(pins),
        _ => println!("There aren't as many motors as you requested"),
    }
}

fn delayHandler() -> u64 {
    println!("Enter delay time in microseconds:");
    let mut delay_str = String::new();
    io::stdin()
        .read_line(&mut delay_str)
        .expect("Failed to read delay");
    let delay_int: u64 = delay_str.trim().parse().expect("Type valid number");
    
    let final_delay = if delay_int > 55 { delay_int - 55 } else { 0 };
    final_delay / 2
}

fn main() {
    println!("Enter motor count you wish to run:");
    let mut choice_str = String::new();
    io::stdin()
        .read_line(&mut choice_str)
        .expect("Failed to read num");
    let count: u8 = choice_str.trim().parse().expect("Type valid int");
    
    multi_motor_handler(count);
}