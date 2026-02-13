import argparse
import time
import RPi.GPIO as GPIO

STEP_PIN = 17
DIR_PIN = 27

def parse_args():
    p = argparse.ArgumentParser(description="Einfaches Stepper-Testskript (DE)")
    p.add_argument(
        "-m", "--motorid",
        type=int,
        choices=range(1, 5),
        default=1,
        dest="motorid",
        help="Motor-Id 1..4 (Standard: 1)",
    )
    p.add_argument(
        "-sc", "--stepcount",
        type=int,
        default=100,
        dest="steps",
        help="Anzahl Schritte (Standard: 100)",
    )
    p.add_argument(
        "-d", "--delay",
        type=float,
        default=0.01,
        dest="delay",
        help="Pause zwischen Schritten in Sekunden (Standard: 0.01)",
    )
    p.add_argument(
        "-dir", "--direction",
        choices=["forward", "backward"],
        default="forward",
        dest="direction",
        help="Richtung: forward|backward (Standard: forward)",
    )
    return p.parse_args()


def setup_gpio():
    GPIO.setmode(GPIO.BCM)
    GPIO.setup(STEP_PIN, GPIO.OUT)
    GPIO.setup(DIR_PIN, GPIO.OUT)
    GPIO.output(STEP_PIN, GPIO.LOW)


def move_motor(steps, delay, direction):
    # Richtung setzen
    if direction == "forward":
        GPIO.output(DIR_PIN, GPIO.HIGH)
    else:
        GPIO.output(DIR_PIN, GPIO.LOW)

    # Schritte erzeugen
    for _ in range(steps):
        GPIO.output(STEP_PIN, GPIO.HIGH)
        time.sleep(delay)
        GPIO.output(STEP_PIN, GPIO.LOW)
        time.sleep(delay)


def main():
    args = parse_args()

    print(f"Motor-ID: {args.motorid}, "
          f"Schritte: {args.steps}, "
          f"Verzögerung: {args.delay}, "
          f"Richtung: {args.direction}")

    setup_gpio()

    try:
        move_motor(args.steps, args.delay, args.direction)
    finally:
        GPIO.cleanup()


if __name__ == "__main__":
    main()