# Homing sensor planning and verification

This worksheet supports [issue #76](https://github.com/HASE-HGV/Roboterarm-HASE/issues/76). It is a record template, not evidence that sensors are installed or homing is safe. Choose and inspect the actual hardware before filling in values; leave unknowns blank rather than assuming a pin, polarity, or sensor type.

## Setup and safety record

| Field | Recorded value |
|---|---|
| Date and operator | |
| Robot / wiring revision | |
| Sensor models and datasheets | |
| Test procedure / risk controls | |
| Emergency-stop method verified before testing | |
| Payload used | |
| Measurement method and resolution | |

## Per-axis sensor and reference definition

The STEP/DIR pins below follow the current README. Select and document a separate Pi input for each home sensor only after checking the actual wiring and GPIO availability.

| Axis | STEP / DIR (BCM) | Sensor model / type | Home input (BCM) | Output interface and active level | Pull-up/down arrangement | Reference position / approach direction | Mounting / hard-stop clearance evidence |
|---|---|---|---|---|---|---|---|
| M3 base | 24 / 25 | | | | | | |
| M1 shoulder | 17 / 27 | | | | | | |
| M2 elbow | 22 / 23 | | | | | | |

## Repeatability results

For each axis, record repeated approaches from both sides of the intended travel, with the arm unloaded and with its normal payload. Use the same documented measurement method and record the individual reference readings, not only an average. Do not run until the sensor mounting, direction, and safety controls have been reviewed.

| Axis | Load (unloaded / normal payload) | Approach side / direction | Trial readings (steps, angle, or measured displacement) | Spread / repeatability | False or missed triggers | Mount shifted? | Pass / notes |
|---|---|---|---|---|---|---|---|
| M3 base | | | | | | | |
| M3 base | | | | | | | |
| M1 shoulder | | | | | | | |
| M1 shoulder | | | | | | | |
| M2 elbow | | | | | | | |
| M2 elbow | | | | | | | |

## Acceptance and sign-off

- [ ] All three active axes have installed sensors and documented, unambiguous home positions.
- [ ] Sensor mounts and approach paths avoid collision with hard stops during normal homing.
- [ ] Sensor input pins, wiring, active levels, and pull-up/down arrangements are recorded and checked.
- [ ] Repeated approaches from both sides trigger consistently for unloaded and normal-payload tests.
- [ ] The recorded spread meets a stated project tolerance for each axis.
- [ ] Sensor positions remain fixed; evidence is attached or linked above.

| Outcome | Reviewer | Date | Outstanding work |
|---|---|---|---|
| Not yet verified | | | |
