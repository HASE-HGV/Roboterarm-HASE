#IMPORTS
import math as m
import time

try:
    import RPi.GPIO as GPIO
    gpio_error = 0
except ImportError:
    print("GPIO nicht verfügbar – Simulation läuft")
    gpio_error = 1

from flask import Flask, request, render_template_string

#VARIABLES
M1_STEP, M1_DIR = 17, 27
M2_STEP, M2_DIR = 22, 23
M3_STEP, M3_DIR = 5, 6
L1, L2 = 500, 525
STEPS_PER_ROUND = 200

#GPIO SETUP
if gpio_error == 0:
    GPIO.setmode(GPIO.BCM)
    for pin in [M1_STEP, M1_DIR, M2_STEP, M2_DIR, M3_STEP, M3_DIR]:
        GPIO.setup(pin, GPIO.OUT)

#CALC ANGLES
def calc_angles(x_mm, z_mm, l1_mm, l2_mm):
    r = m.sqrt(x_mm**2 + z_mm**2)
    if r > l1_mm + l2_mm:
        raise ValueError("Position außerhalb des Arbeitsbereichs")
    alpha = m.atan2(z_mm, x_mm)
    cos_theta2 = max(-1, min(1, (r**2 - l1_mm**2 - l2_mm**2)/(2*l1_mm*l2_mm)))
    theta2 = m.acos(cos_theta2)
    theta1 = alpha - m.atan2(l2_mm*m.sin(theta2), l1_mm+l2_mm*m.cos(theta2))
    return m.degrees(theta1), m.degrees(theta2)

#STEP FUNCTION
def step(STEP_PIN, DIR_PIN, direction):
    if gpio_error == 0:
        GPIO.output(DIR_PIN, GPIO.HIGH if direction else GPIO.LOW)
        GPIO.output(STEP_PIN, GPIO.HIGH)
        GPIO.output(STEP_PIN, GPIO.LOW)
    else:
        print(f"Step on {STEP_PIN}, dir: {'+' if direction else '-'}")



app = Flask(__name__)
 
#HTML IS VIBE CODED DA SONST SIEHT NIX GUT AUS

HTML = """<!DOCTYPE html>
<html lang="de">
<head>
<meta charset="UTF-8">
<title>Roboterarm Steuerung</title>
<style>
body {font-family: Arial, sans-serif; background-color: #f0f0f0; display:flex; flex-direction:column; align-items:center; padding:30px;}
h2 {color:#333;}
form {background:#fff; padding:20px 30px; border-radius:10px; box-shadow:0 2px 10px rgba(0,0,0,0.1);}
label,input {display:block; margin:10px 0;}
input[type=number]{width:100px; padding:5px;}
input[type=submit]{margin-top:15px; padding:10px 20px; border:none; border-radius:5px; background-color:#4CAF50; color:white; cursor:pointer;}
input[type=submit]:hover{background-color:#45a049;}
pre {background-color:#222; color:#0f0; padding:10px; width:600px; overflow-x:auto; margin-top:20px; border-radius:5px; text-align:left;}
table {margin-top:10px; border-collapse: collapse;}
td, th {border: 1px solid #333; padding:5px;}
</style>
</head>
<body>
<h2>Roboterarm Steuerung (Koordinaten)</h2>
<form method="post">
<label>Start X: <input type="number" name="sx" value="100"></label>
<label>Start Z: <input type="number" name="sz" value="300"></label>
<label>Start Basis-Rotation Y: <input type="number" name="sy" value="0"></label>
<label>End X: <input type="number" name="ex" value="400"></label>
<label>End Z: <input type="number" name="ez" value="600"></label>
<label>End Basis-Rotation Y: <input type="number" name="ey" value="90"></label>
<label>Zeit (Sekunden): <input type="number" name="time" value="5"></label>
<input type="submit" value="Bewegen">
</form>

<h3>Empfangene Daten:</h3>
{% if received %}
<table>
<tr><th>Feld</th><th>Wert</th></tr>
{% for key, val in received.items() %}
<tr><td>{{ key }}</td><td>{{ val }}</td></tr>
{% endfor %}
</table>
{% endif %}

<pre>{{ output }}</pre>
</body>
</html>
"""

def ease_in_out(t):
    return 0.5 - 0.5*m.cos(m.pi*t)

@app.route("/", methods=["GET","POST"])
def index():
    output = ""
    received = {}
    if request.method == "POST":
        try:
            sx = float(request.form["sx"])
            sz = float(request.form["sz"])
            sy = float(request.form["sy"])
            ex = float(request.form["ex"])
            ez = float(request.form["ez"])
            ey = float(request.form["ey"])
            total_time = float(request.form["time"])
            
            received = request.form.to_dict()
            
            steps_count = 50
            prev_theta1, prev_theta2 = calc_angles(sx, sz, L1, L2)
            prev_rot = sy
            
            # Gesamtdeltas für jeden Motor
            total_delta_rot = ey - sy
            total_delta_theta1 = 0
            total_delta_theta2 = 0
            # Berechne Deltas iterativ
            x, z = sx, sz
            for i in range(1, steps_count+1):
                t_linear = i/steps_count
                t = ease_in_out(t_linear)
                xi = sx + t*(ex - sx)
                zi = sz + t*(ez - sz)
                theta1_i, theta2_i = calc_angles(xi, zi, L1, L2)
                total_delta_theta1 += theta1_i - prev_theta1
                total_delta_theta2 += theta2_i - prev_theta2
                prev_theta1, prev_theta2 = theta1_i, theta2_i

            # Jetzt exakte Steps pro Motor
            steps_m1_total = int(round(abs(total_delta_rot)/360*STEPS_PER_ROUND))
            steps_m2_total = int(round(abs(total_delta_theta1)/360*STEPS_PER_ROUND))
            steps_m3_total = int(round(abs(total_delta_theta2)/360*STEPS_PER_ROUND))
            dir_m1 = total_delta_rot >= 0
            dir_m2 = total_delta_theta1 >= 0
            dir_m3 = total_delta_theta2 >= 0
            
            # Step-Zähler
            counter_m1, counter_m2, counter_m3 = 0, 0, 0
            acc_m1 = acc_m2 = acc_m3 = 0  # Akkumulatoren für proportionalen Step

            # Parallele Step-Ausführung
            max_steps = max(steps_m1_total, steps_m2_total, steps_m3_total)
            for s in range(max_steps):
                # M1
                acc_m1 += steps_m1_total
                if acc_m1 >= max_steps:
                    step(M1_STEP, M1_DIR, dir_m1)
                    counter_m1 += 1
                    acc_m1 -= max_steps

                # M2
                acc_m2 += steps_m2_total
                if acc_m2 >= max_steps:
                    step(M2_STEP, M2_DIR, dir_m2)
                    counter_m2 += 1
                    acc_m2 -= max_steps

                # M3
                acc_m3 += steps_m3_total
                if acc_m3 >= max_steps:
                    step(M3_STEP, M3_DIR, dir_m3)
                    counter_m3 += 1
                    acc_m3 -= max_steps

                output += f"Step-Zähler: M1={counter_m1}/{steps_m1_total}, M2={counter_m2}/{steps_m2_total}, M3={counter_m3}/{steps_m3_total}\n"
                time.sleep(total_time/max_steps)
                
        except Exception as e:
            output += f"Fehler: {e}"
    
    return render_template_string(HTML, output=output, received=received)

if __name__=="__main__":
    app.run(host="0.0.0.0", port=5000)