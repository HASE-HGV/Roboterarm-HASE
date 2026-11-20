# A4988 driver verification record

Use this worksheet to record the physical checks required by [issue #77](https://github.com/HASE-HGV/Roboterarm-HASE/issues/77). Fill it in from the installed boards and measurements; do not infer clone-board values from another driver. Empty fields mean the check has not been completed.

## Inspection and measurement conditions

| Field | Recorded value |
|---|---|
| Date and operator | |
| Robot revision / wiring revision | |
| Motor supply voltage | |
| Measuring instrument(s) and model | |
| Ambient temperature | |
| Payload used for loaded run | |

## Per-driver inspection

Record the resistor marking and derive its value from the specific board before calculating current limit. Photograph the board markings and capacitor placement so the record can be reviewed later.

| Driver / axis | Board marking or revision | Sense resistor marking | Rsense (Ω) | Capacitor measured / rated (µF / V) | Polarity correct | Close to VMOT/GND | Photo / evidence |
|---|---|---|---:|---|---|---|---|
| M1 shoulder | | | | | | | |
| M2 elbow | | | | | | | |
| M3 base | | | | | | | |
| M4 reserved | | | | | | | |

## Current-limit record

Calculate each target only after identifying that board's Rsense and confirming the motor's rated phase current from its label or datasheet. Record the calculation and source; do not copy a Vref from another axis.

| Driver / axis | Motor model | Rated phase current (A) and source | Rsense (Ω) and source | Target Vref calculation | Measured Vref (V) | Calculated current limit (A) | Within motor rating? |
|---|---|---|---|---|---:|---:|---|
| M1 shoulder | | | | | | | |
| M2 elbow | | | | | | | |
| M3 base | | | | | | | |
| M4 reserved | | | | | | | |

## Run checks

Record unloaded and representative-load observations for every active axis. M4 is reserved; document its electrical checks above, and mark motion testing N/A unless it is deliberately configured for a controlled test.

| Axis | Run type (unloaded / representative load) | Duration | Direction changes / deceleration tested | Driver temperature / method | Motor temperature / method | Reset or dropout observed? | Result / notes |
|---|---|---:|---|---|---|---|---|
| M1 shoulder | | | | | | | |
| M1 shoulder | | | | | | | |
| M2 elbow | | | | | | | |
| M2 elbow | | | | | | | |
| M3 base | | | | | | | |
| M3 base | | | | | | | |
| M4 reserved | N/A unless configured | | | | | | |

## Sign-off

- [ ] All four VMOT capacitors are present, at least 100 µF, correctly polarized, and mounted close to VMOT/GND.
- [ ] The capacitor voltage rating is recorded and suitable for the project's 12 V motor rail.
- [ ] Rsense was identified separately on every driver before calculating each current limit.
- [ ] Each active motor's measured current limit is within its documented rating.
- [ ] Unloaded and representative-load checks passed without overheating or driver resets.
- [ ] Evidence (photos, readings, and notes) is attached or linked above.

| Outcome | Reviewer | Date | Outstanding issues |
|---|---|---|---|
| Not yet verified | | | |
