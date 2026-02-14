import argparse
import time
import RPi.GPIO as GPIO
<<<<<<< Updated upstream
import math as m



MOTOR_PINS = {
    1: {"STEP": 17, "DIR": 27},
    2: {"STEP": 22, "DIR": 23},
    3: {"STEP": 24,  "DIR": 25},
    4: {"STEP": 5, "DIR": 6},
}



def parse_args():
    p = argparse.ArgumentParser(description="Stepper-Testskript (4 Motoren)")
    p.add_argument(
        "-m",
        "--motorid",
=======

STEP_PIN = 17
DIR_PIN = 27

def parse_args():
    p = argparse.ArgumentParser(description="Einfaches Stepper-Testskript (DE)")
    p.add_argument(
        "-m", "--motorid",
>>>>>>> Stashed changes
        type=int,
        choices=range(1, 5),
        default=1,
        dest="motorid",
        help="Motor-Id 1..4 (Standard: 1)",
    )
    p.add_argument(
<<<<<<< Updated upstream
        "-sc",
        "--stepcount",
        type=int,
        default=200,
        dest="steps",
        help="Anzahl Schritte (Standard: 200)",
    )
    p.add_argument(
        "-d",
        "--delay",
        type=float,
        default=0.001,
        dest="delay",
        help="Pause zwischen Schritten in Sekunden",
    )
    p.add_argument(
        "-dir",
        "--direction",
        choices=["forward", "backward"],
        default="forward",
        dest="direction",
        help="Richtung: forward|backward",
=======
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
>>>>>>> Stashed changes
    )
    return p.parse_args()


<<<<<<< Updated upstream

def setup_gpio():
    GPIO.setmode(GPIO.BCM)

    for motor in MOTOR_PINS.values():
        GPIO.setup(motor["STEP"], GPIO.OUT)
        GPIO.setup(motor["DIR"], GPIO.OUT)
        GPIO.output(motor["STEP"], GPIO.LOW)



def move_motor(motor_id, steps, delay, direction):

    step_pin = MOTOR_PINS[motor_id]["STEP"]
    dir_pin = MOTOR_PINS[motor_id]["DIR"]

    
    if direction == "forward":
        GPIO.output(dir_pin, GPIO.LOW)
    else:
        GPIO.output(dir_pin, GPIO.HIGH)

    
    for _ in range(steps):
        GPIO.output(step_pin, GPIO.HIGH)
        time.sleep(delay)
        GPIO.output(step_pin, GPIO.LOW)
=======
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
>>>>>>> Stashed changes
        time.sleep(delay)


def main():
    args = parse_args()

<<<<<<< Updated upstream
    print(
        f"Motor-ID: {args.motorid}, "
        f"Schritte: {args.steps}, "
        f"Delay: {args.delay}, "
        f"Richtung: {args.direction}"
    )
=======
    print(f"Motor-ID: {args.motorid}, "
          f"Schritte: {args.steps}, "
          f"Verzögerung: {args.delay}, "
          f"Richtung: {args.direction}")
>>>>>>> Stashed changes

    setup_gpio()

    try:
<<<<<<< Updated upstream
        move_motor(args.motorid, args.steps, args.delay, args.direction)
=======
        move_motor(args.steps, args.delay, args.direction)
>>>>>>> Stashed changes
    finally:
        GPIO.cleanup()


if __name__ == "__main__":
<<<<<<< Updated upstream
    main()
=======
    main()
>>>>>>> Stashed changes
