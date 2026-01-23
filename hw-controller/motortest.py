#!/usr/bin/env python3
"""
Einfaches, vereinfachtes Testskript für Stepper-Motoren (deutsch).

Beispiel:
  python motortest.py -m 1 -s SINGLE -c 100 -d 0.01 -D vor
  python motortest.py --simulieren -m 2 -s micro -c 50


 `--simulieren`-Option für Systeme ohne Hardware.
"""

import argparse
import time
import sys


# Minimale `stepper`-Konstanten global definieren, damit sie immer verfügbar sind.
# Bei vorhandenem Hardware-Modul können diese überschrieben oder angepasst werden.
class stepper:
    SINGLE = "SINGLE"
    DOUBLE = "DOUBLE"
    MICROSTEP = "MICRO"
    FORWARD = "FORWARD"
    BACKWARD = "BACKWARD"


# Versuche Hardware-Module zu laden; bei Fehlschlag Simulation anbieten
try:
    import board
    import busio
    from adafruit_motorkit import MotorKit
    import adafruit_motor
    import adafruit_motorkit

    HARDWARE = True
except Exception:
    HARDWARE = False

    # Hardware-Module konnten nicht importiert werden. Die globale
    # `stepper`-Klasse oben enthält bereits minimale Konstanten, daher
    # ist hier keine zusätzliche Definition notwendig.


def parse_args():
    p = argparse.ArgumentParser(description="Einfaches Stepper-Testskript (DE)")
    p.add_argument(
        "-s",
        "--stepstyle",
        choices=["SINGLE", "double", "micro"],
        default="DOUBLE",
        dest="stepstyle",
        help="Schritt-Modus: einfach|doppel|micro (Standard: einfach)",
    )
    p.add_argument(
        "-m",
        "--motorid",
        type=int,
        choices=range(1, 5),
        default=1,
        dest="motorid",
        help="Motor-Id 1..4 (Standard: 1)",
    )
    p.add_argument(
        "-sc",
        "--stepcount",
        type=int,
        default=100,
        dest="steps",
        help="Anzahl Schritte (Standard: 100)",
    )
    # Verwende -v für Verzögerung, um Konflikt mit -d zu vermeiden
    p.add_argument(
        "-d",
        "--delay",
        type=float,
        default=0.01,
        dest="delay",
        help="Pause zwischen Schritten in Sekunden (Standard: 0.01)",
    )
    p.add_argument(
        "-dir",
        "--direction",
        choices=["forward", "backward"],
        default="forward",
        dest="direction",
        help="Richtung: vor|zurueck (Standard: vor)",
    )
    p.add_argument(
        "--simulate",
        action="store_true",
        dest="simulieren",
        help="Ohne Hardware laufen (nur Ausgabe)",
    )
    return p.parse_args()


def build_motors():
    i2c = busio.I2C(board.SCL, board.SDA)
    kit1 = MotorKit(i2c=i2c, address=0x60)
    kit2 = MotorKit(i2c=i2c, address=0x70)

    return [kit1.stepper1, kit1.stepper2, kit2.stepper1, kit2.stepper2]


def main():
    args = parse_args()

    if args.simulieren:
        print("Simulation: keine Hardware-Operationen")
    elif not HARDWARE:
        print(
            "Hardware-Module nicht verfügbar. Starte mit --simulate, wenn nötig.",
            file=sys.stderr,
        )
        sys.exit(1)

    sstyle_map = {
        "single": stepper.SINGLE,
        "double": stepper.DOUBLE,
        "micro": stepper.MICROSTEP,
    }
    dir_map = {"forward": stepper.FORWARD, "backward": stepper.BACKWARD}

    sstyle = sstyle_map[args.stepstyle]
    m_direction: str = dir_map[args.direction]
    idx = args.motorid - 1

    motor = None
    if not args.simulieren:
        motors = build_motors()
        if not (0 <= idx < len(motors)):
            print(f"Ungueltige Motor-Id: {args.motorid}", file=sys.stderr)
            sys.exit(1)
        motor = motors[idx]

    # Zusätzliche Laufzeitprüfung: stelle sicher, dass motor initialisiert wurde
    if not args.simulieren and motor is None:
        print("Fehler: Motor konnte nicht initialisiert werden.", file=sys.stderr)
        sys.exit(1)

    print(
        f"Start: motor={args.motorid} schritte={args.schritte} stil={args.stepstil} "
        f"richtung={args.richtung} verz={args.verzoegerung} simul={args.simulieren}"
    )

    try:
        for i in range(args.schritte):
            if args.simulieren:
                # Nur bei jedem 10%-Schritt etwas ausgeben
                if args.schritte >= 10 and (i % (max(1, args.schritte // 10)) == 0):
                    print(f"Simulierter Schritt {i+1}/{args.schritte}")
            else:
                motor.onestep(
                    direction=m_direction, style=sstyle
                )  # >>>>> Wenn hier was steht von wegen onestep is not defined dann kann man das meistens ignorieren
            time.sleep(args.verzoegerung)
    except KeyboardInterrupt:
        print("\nAbgebrochen vom Benutzer")
    finally:
        if motor is not None:
            rel = getattr(motor, "release", None)
            if callable(rel):
                rel()
        print("Fertig")


if __name__ == "__main__":
    main()
