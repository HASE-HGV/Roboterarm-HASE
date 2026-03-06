try:
    import RPi.GPIO as GPIO  # Import GPIO Pins
except ImportError:
    import Mock.GPIO as GPIO  # If import fails -> Import Mock GPIO for Simulation

GPIO.setmode(GPIO.BCM)  # Setting GPIO Mode -> BCM OR BOARD


def setup_pins(pins):
    for p in pins:
        GPIO.setup(p, GPIO.OUT)


def cleanup():
    GPIO.cleanup()
