try:
    import RPi.GPIO as GPIO
except ImportError:
    import Mock.GPIO as GPIO

GPIO.setmode(GPIO.BCM)


def setup_pins(pins):
    for p in pins:
        GPIO.setup(p, GPIO.OUT)


def cleanup():
    GPIO.cleanup()
