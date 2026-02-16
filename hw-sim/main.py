import argparse
from config import *
from gpio_setup import setup_pins, cleanup
from ik import calc_angles
from motor import move_motor
import math as m


parser = argparse.ArgumentParser(description="Roboter IK Bewegung")

parser.add_argument("--sx", type=float, required=True)
parser.add_argument("--sy", type=float, required=True)
parser.add_argument("--sz", type=float, required=True)

parser.add_argument("--ex", type=float, required=True)
parser.add_argument("--ey", type=float, required=True)
parser.add_argument("--ez", type=float, required=True)

parser.add_argument("--time", type=float, required=True)

args = parser.parse_args()

setup_pins([M1_STEP, M1_DIR, M2_STEP, M2_DIR, M3_STEP, M3_DIR])


start_theta1_deg, start_theta2_deg = calc_angles(
    args.sx, args.sz, LENGTH_ARM_1_MM, LENGTH_ARM_2_MM
)


end_theta1_deg, end_theta2_deg = calc_angles(
    args.ex, args.ez, LENGTH_ARM_1_MM, LENGTH_ARM_2_MM
)

delta_rot_deg = args.ey - args.sy
delta_schulter_deg = end_theta1_deg - start_theta1_deg
delta_ellenbogen_deg = end_theta2_deg - start_theta2_deg

m1_steps = int(abs(delta_rot_deg) / 360 * STEPS_PER_ROUND)
m2_steps = int(abs(delta_schulter_deg) / 360 * STEPS_PER_ROUND)
m3_steps = int(abs(delta_ellenbogen_deg) / 360 * STEPS_PER_ROUND)

if args.time <= 0:
    raise ValueError("Bewegungszeit muss größer als 0 sein")

m1_delay = args.time / (m1_steps * 2) if m1_steps else 0
m2_delay = args.time / (m2_steps * 2) if m2_steps else 0
m3_delay = args.time / (m3_steps * 2) if m3_steps else 0


try:
    move_motor(M1_STEP, M1_DIR, m1_steps, m1_delay, delta_rot_deg)
    move_motor(M2_STEP, M2_DIR, m2_steps, m2_delay, delta_schulter_deg)
    move_motor(M3_STEP, M3_DIR, m3_steps, m3_delay, delta_ellenbogen_deg)
finally:
    cleanup()
