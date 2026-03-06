import RPi.GPIO as GPIO
import time

# GPIO-Pins festlegen
STEP_PIN = 17
DIR_PIN = 27

# GPIO-Modus auf BCM setzen
GPIO.setmode(GPIO.BCM)

# GPIO-Pins als Ausgang definieren
GPIO.setup(STEP_PIN, GPIO.OUT)
GPIO.setup(DIR_PIN, GPIO.OUT)

# Die Richtung des Motors festlegen (True für eine Richtung, False für die andere)
GPIO.output(
    DIR_PIN, GPIO.HIGH
)  # HIGH für eine Richtung (z.B. Uhrzeigersinn), LOW für die andere Richtung

# Anzahl der Schritte pro Umdrehung des Motors (abhängig vom Steppermotor)
steps_per_revolution = 200  # Beispiel: 200 Schritte pro Umdrehung für einen 1,8°-Motor

# Schrittweite (Zeit zwischen den einzelnen Schritten)
step_delay = 0.001  # 1 ms zwischen den Schritten

try:
    while True:
        # Motor schrittweise bewegen
        for _ in range(steps_per_revolution):
            GPIO.output(STEP_PIN, GPIO.HIGH)
            time.sleep(step_delay)
            GPIO.output(STEP_PIN, GPIO.LOW)
            time.sleep(step_delay)

        # Optional: Pause zwischen den Bewegungen
        time.sleep(1)

except KeyboardInterrupt:
    print("Programm beendet.")
    GPIO.cleanup()  # Alle GPIO-Einstellungen zurücksetzen
