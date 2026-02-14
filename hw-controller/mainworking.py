import sys

try:
    import math as m
except ImportError:
    print("MATH konnten nicht initialisiert werden")
    sys.exit(1)

try:
    import time
except ImportError:
    print("TIME konnten nicht initialisiert werden")
    sys.exit(1)

try:
    import argparse
except ImportError:
    print("ARGPARSE konnten nicht initialisiert werden")
    sys.exit(1)

try:
    import RPi.GPIO as GPIO
except ImportError:
    import Mock.GPIO as GPIO


M1_STEP = 17
M1_DIR = 27

M2_STEP = 22
M2_DIR = 23

M3_STEP = 5
M3_DIR = 6

parser = argparse.ArgumentParser(description="Roboter IK Bewegung")

parser.add_argument("--sx", type=float, required=True)
parser.add_argument("--sy", type=float, required=True)
parser.add_argument("--sz", type=float, required=True)

parser.add_argument("--ex", type=float, required=True)
parser.add_argument("--ey", type=float, required=True)
parser.add_argument("--ez", type=float, required=True)

parser.add_argument("--time", type=float, required=True)

args = parser.parse_args()

# Arguments for Calculations

length_arm_1_mm = 500
length_arm_2_mm = 525
stps_per_round = 200

start_pos_x_mm = args.sx
start_pos_y_degrees = args.sy
start_pos_z_mm = args.sz

end_pos_x_mm = args.ex
end_pos_y_degrees = args.ey
end_pos_z_mm = args.ez

mov_time_sek = args.time

GPIO.setmode(GPIO.BCM)

GPIO.setup(M1_STEP, GPIO.OUT)
GPIO.setup(M1_DIR, GPIO.OUT)

GPIO.setup(M2_STEP, GPIO.OUT)
GPIO.setup(M2_DIR, GPIO.OUT)

GPIO.setup(M3_STEP, GPIO.OUT)
GPIO.setup(M3_DIR, GPIO.OUT)

# Calculation Start


start_distance_r_mm = m.sqrt(start_pos_x_mm**2 + start_pos_z_mm**2)

if start_distance_r_mm > (length_arm_1_mm + length_arm_2_mm):
    raise ValueError("Startposition außerhalb des Arbeitsbereichs")

start_alpha_rad = m.atan2(start_pos_z_mm, start_pos_x_mm)

start_cos_theta2 = (
    start_distance_r_mm**2 - length_arm_1_mm**2 - length_arm_2_mm**2
) / (2 * length_arm_1_mm * length_arm_2_mm)

start_cos_theta2 = max(-1, min(1, start_cos_theta2))

start_theta2_rad = m.acos(start_cos_theta2)

start_theta1_rad = start_alpha_rad - m.atan2(
    length_arm_2_mm * m.sin(start_theta2_rad),
    length_arm_1_mm + length_arm_2_mm * m.cos(start_theta2_rad),
)

start_theta1_deg = m.degrees(start_theta1_rad)
start_theta2_deg = m.degrees(start_theta2_rad)

# Calculation End

end_distance_r_mm = m.sqrt(end_pos_x_mm**2 + end_pos_z_mm**2)

if end_distance_r_mm > (length_arm_1_mm + length_arm_2_mm):
    raise ValueError("Endposition außerhalb des Arbeitsbereichs")

end_alpha_rad = m.atan2(end_pos_z_mm, end_pos_x_mm)

end_cos_theta2 = (
    end_distance_r_mm**2 - length_arm_1_mm**2 - length_arm_2_mm**2
) / (2 * length_arm_1_mm * length_arm_2_mm)

end_cos_theta2 = max(-1, min(1, end_cos_theta2))

end_theta2_rad = m.acos(end_cos_theta2)

end_theta1_rad = end_alpha_rad - m.atan2(
    length_arm_2_mm * m.sin(end_theta2_rad),
    length_arm_1_mm + length_arm_2_mm * m.cos(end_theta2_rad),
)

end_theta1_deg = m.degrees(end_theta1_rad)
end_theta2_deg = m.degrees(end_theta2_rad)

delta_rot_deg = end_pos_y_degrees - start_pos_y_degrees
delta_schulter_deg = end_theta1_deg - start_theta1_deg
delta_ellenbogen_deg = end_theta2_deg - start_theta2_deg

m1_steps = int(abs(delta_rot_deg) / 360 * stps_per_round)
m2_steps = int(abs(delta_schulter_deg) / 360 * stps_per_round)
m3_steps = int(abs(delta_ellenbogen_deg) / 360 * stps_per_round)

if mov_time_sek <= 0:
    raise ValueError("Bewegungszeit muss größer als 0 sein")

m1_delay = mov_time_sek / (m1_steps * 2) if m1_steps != 0 else 0
m2_delay = mov_time_sek / (m2_steps * 2) if m2_steps != 0 else 0
m3_delay = mov_time_sek / (m3_steps * 2) if m3_steps != 0 else 0


def move_motor(step_pin, dir_pin, steps, delay, delta):
    if delta >= 0:
        GPIO.output(dir_pin, GPIO.LOW)
    else:
        GPIO.output(dir_pin, GPIO.HIGH)

    for _ in range(steps):
        GPIO.output(step_pin, GPIO.HIGH)
        time.sleep(delay)
        GPIO.output(step_pin, GPIO.LOW)
        time.sleep(delay)

        # Löscht die vorherige Ausgabe und überschreibt sie
        print(
            f"\rSTEP_PIN: {step_pin}, Steps: {steps},Current_STEP: {_}, Delay: {delay:.4f}s, Delta: {delta:.2f}°---------",
            end="",  # Verhindert eine neue Zeile
        )


try:
    move_motor(M1_STEP, M1_DIR, m1_steps, m1_delay, delta_rot_deg)
    move_motor(M2_STEP, M2_DIR, m2_steps, m2_delay, delta_schulter_deg)
    move_motor(M3_STEP, M3_DIR, m3_steps, m3_delay, delta_ellenbogen_deg)
finally:
    GPIO.cleanup()
