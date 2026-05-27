use std::{sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}}, time::Duration, thread};
use rppal::gpio::{Gpio};
use ctrlc;

const time:u64 = 500;
const GPIO_PIN: u8 = 17;
const BLINK_INTERVAL: Duration = Duration::from_micros(time - 55);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Pin initialisieren
    let pin = Gpio::new()?.get(GPIO_PIN)?.into_output();
    
    // Wir nutzen Arc/Mutex für den Zugriff von zwei Stellen (Main-Loop & Handler)
    let shared = Arc::new(Mutex::new(Some(pin)));
    
    // Ein Flag, um der Loop zu sagen, dass sie aufhören soll
    let terminate = Arc::new(AtomicBool::new(false));

    // 2. Ctrl-C Handler einrichten
    {
        let s = Arc::clone(&shared);
        let t = Arc::clone(&terminate);
        
        ctrlc::set_handler(move || {
            // Signalisiere der Loop das Ende
            t.store(true, Ordering::SeqCst);
            
            // Versuche, den Pin sofort auf 'Low' zu setzen
            if let Ok(mut guard) = s.lock() {
                if let Some(ref mut p) = *guard {
                    let _ = p.set_low(); 
                }
            }
            // Wir rufen hier NICHT exit(0) auf, damit der Hauptthread 
            // sauber aufräumen kann (Drop).
        })?;
    }

    println!("Blinking on pin {}. Press Ctrl+C to stop.", GPIO_PIN);

    // 3. Haupt-Loop
    loop {
        // Prüfen, ob Ctrl+C gedrückt wurde
        if terminate.load(Ordering::SeqCst) {
            println!("\nCleaning up...");
            if let Ok(mut guard) = shared.lock() {
                if let Some(mut p) = guard.take() {
                    let _ = p.set_low();
                    // p wird hier gedroppt -> GPIO wird freigegeben
                }
            }
            break; 
        }

        // Normales Blinken
        if let Ok(mut guard) = shared.lock() {
            if let Some(ref mut p) = *guard {
                p.toggle();
            }
        }

        thread::sleep(BLINK_INTERVAL);
    }

    Ok(())
}
