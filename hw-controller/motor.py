import time
from gpio_setup import GPIO


def move_motor(step_pin, dir_pin, steps, delay, delta):
    if delta >= 0:
        GPIO.output(dir_pin, GPIO.LOW)
    else:
        GPIO.output(dir_pin, GPIO.HIGH)

    for i in range(steps):
        GPIO.output(step_pin, GPIO.HIGH)
        time.sleep(delay)
        GPIO.output(step_pin, GPIO.LOW)
        time.sleep(delay)

        print(
            f"\rSTEP_PIN: {step_pin}, Steps: {steps}, "
            f"Current_STEP: {i}, Delay: {delay:.4f}s, "
            f"Delta: {delta:.2f}°---------",
            end="",
        )
