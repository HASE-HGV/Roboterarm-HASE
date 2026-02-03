## Requirements

- Rust und Cargo (https://rustup.rs/)
- Für Hardware: I2C-Libraries

## Installation

```bash
# Rust installieren falls noch nicht da
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Project builden
cargo build --release
```

## Usage

### Simulation (ohne Hardware)
```bash
cargo run -- --simulate -m 1 -s single -c 100 -d 0.01 --direction forward
cargo run -- --simulate -m 2 -s micro -c 50
```

### Mit Hardware
```bash
cargo run -- -m 1 -s single -c 100 -d 0.01 --direction forward
cargo run -- -m 2 -s double -c 200 -d 0.02 --direction backward
```

## Command-Line Args

- `-m, --motorid <ID>`: Motor-ID (1-4), default: 1
- `-s, --stepstyle <STYLE>`: Step-Mode (single|double|micro), default: double
- `-c, --stepcount <COUNT>`: Anzahl Steps, default: 100
- `-d, --delay <SECS>`: Delay zwischen Steps, default: 0.01
- `--direction <DIR>`: Direction (forward|backward), default: forward
- `--simulate`: Simulation-Mode ohne Hardware

### Code-Improvements:

- Enums statt String-Constants
- Klare Function-Separation
- Explicit Error-Types
- Pattern Matching statt Dictionary-Lookups
- Command-Line Args werden compile-time validiert

## Hardware-Integration

Aktuelle Version hat nur Placeholders für Hardware.
Für echte Hardware musst du:

1. I2C-Library einbinden (z.B. `rppal` für Raspberry Pi)
2. `build_motors()` und `StepperMotor::onestep()` implementieren
3. Driver für Adafruit MotorKit porten

## Binary ausführen

```bash
# Nach dem Build
# im root von picontrol
./target/release/stepper-motor-test --help
./target/release/stepper-motor-test --simulate -m 1 -c 100
```

## Quick Start

```bash
# Project setup
cargo build --release 

# Testen im Simulation-Mode
cargo run -- --simulate -m 1 -c 50

# Help
cargo run -- --help
```
