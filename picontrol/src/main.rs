#!/usr/bin/env rust-script

use std::thread;
use std::time::Duration;
use clap::Parser;

/// Schritt-Modi für Stepper-Motoren
#[derive(Debug, Clone, Copy)]
enum StepStyle {
    Single,
    Double,
    Micro,
}

impl StepStyle {
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "single" => Some(StepStyle::Single),
            "double" => Some(StepStyle::Double),
            "micro" => Some(StepStyle::Micro),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Direction {
    Forward,
    Backward,
}

impl Direction {
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "forward" => Some(Direction::Forward),
            "backward" => Some(Direction::Backward),
            _ => None,
        }
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// single|double|micro
    #[arg(short = 's', long, default_value = "double")]
    stepstyle: String,

    /// Motor-Id 1..4,
    #[arg(short = 'm', long, default_value_t = 1)]
    motorid: u8,

    /// Anzahl Steps(100 (180°)Preset)
    #[arg(short = 'c', long = "stepcount", default_value_t = 100)]
    steps: usize,

    /// Step-Gap in s(0.01 Preset)
    #[arg(short = 'd', long, default_value_t = 0.01)]
    delay: f64,

    /// Richtung: forward|backward (forward Preset)
    #[arg(long, default_value = "forward")]
    direction: String,

    /// Simulator(nur Ausgabe)
    #[arg(long)]
    simulate: bool,
}

struct StepperMotor {
    id: u8,
}

impl StepperMotor {
    fn new(id: u8) -> Self {
        StepperMotor { id }
    }

    /// Führt einen einzelnen Schritt aus
    fn onestep(&self, direction: Direction, style: StepStyle) {
        // Hier würde die tatsächliche Hardware-Ansteuerung erfolgen
        println!(
            "Motor {} führt Schritt aus: {:?} in Richtung {:?}",
            self.id, style, direction
        );
    }

    /// Motor Stromlos (Notstopp?)
    fn release(&self) {
        println!("Motor {} wird freigegeben", self.id);
    }
}

/// Baut die Motor-Objekte auf (Platzhalter für Hardware-Initialisierung)
fn build_motors() -> Result<Vec<StepperMotor>, String> {
    // In einer echten Implementierung würde hier die I2C-Verbindung
    // zu den MotorKit-Boards (Adressen 0x60 und 0x70) aufgebaut werden
    
    Ok(vec![
        StepperMotor::new(1),
        StepperMotor::new(2),
        StepperMotor::new(3),
        StepperMotor::new(4),
    ])
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Args = Args::parse();

    // Validierung der Motor-ID
    if !(1..=4).contains(&args.motorid) {
        eprintln!("Ungueltige Motor-Id: {}", args.motorid);
        std::process::exit(1);
    }

    // Parse Schritt-Stil
    let step_style = StepStyle::from_str(&args.stepstyle)
        .ok_or_else(|| format!("Ungueltiger Schritt-Stil: {}", args.stepstyle))?;

    // Parse Richtung
    let direction: Direction = Direction::from_str(&args.direction)
        .ok_or_else(|| format!("Ungueltige Richtung: {}", args.direction))?;

    // Simulation oder Hardware-Modus
    if args.simulate {
        println!("Simulation: keine Hardware-Operationen");
    }
    else if !args.simulate {
        println!("Currently not implemented!")
    }

    // Motor initialisieren (nur wenn kein Simulant lol)
    let motor: Option<StepperMotor> = if !args.simulate {
        let motors: Vec<StepperMotor> = build_motors()?;
        let idx: usize = (args.motorid - 1) as usize;
        
        if idx >= motors.len() {
            eprintln!("Ungueltige Motor-Id: {}", args.motorid);
            std::process::exit(1);
        }
        
        Some(motors.into_iter().nth(idx).unwrap())
    } else {
        None
    };

    println!(
        "Start: motor={} schritte={} stil={:?} richtung={:?} verz={} simul={}",
        args.motorid, args.steps, step_style, direction, args.delay, args.simulate
    );

    let result: Result<(), String> = run_motor_steps(
        motor,
        args.steps,
        args.delay,
        step_style,
        direction,
        args.simulate,
    );

    match result {
        Ok(_) => println!("Fertig"),
        Err(e) => eprintln!("Fehler: {}", e),
    }

    Ok(())
}

/// Step-Runner
fn run_motor_steps(
    motor: Option<StepperMotor>,
    steps: usize,
    delay: f64,
    style: StepStyle,
    direction: Direction,
    simulate: bool,
) -> Result<(), String> {
    let delay_duration: Duration = Duration::from_secs_f64(delay);
    let progress_interval: usize = if steps >= 10 { steps / 10 } else { 1 };

    for i in 0..steps {
        if simulate {
            // Nur bei jedem 10%-Schritt etwas ausgeben
            if i % progress_interval == 0 {
                println!("Simulierter Schritt {}/{}", i + 1, steps);
            }
        } else {
            if let Some(ref m) = motor {
                m.onestep(direction, style);
            } else {
                return Err("Motor nicht initialisiert".to_string());
            }
        }
        
        thread::sleep(delay_duration);
    }

    // Motor freigeben wenn vorhanden
    if let Some(ref m) = motor {
        m.release();
    }

    Ok(())
}
