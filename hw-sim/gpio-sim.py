import tkinter as tk
import threading
import sys
import types
import Mock.GPIO as GPIO
from datetime import datetime

# ---------------- GPIO PATCH ----------------

_callbacks = []
_original_output = GPIO.output
_original_input = getattr(GPIO, "input", lambda pin: False)
_pin_states = {}


def patched_output(pin, state):
    _pin_states[pin] = state
    for cb in _callbacks:
        cb(pin, state)
    _original_output(pin, state)


def patched_input(pin):
    return _pin_states.get(pin, False)


GPIO.output = patched_output
GPIO.input = patched_input


def register_callback(cb):
    _callbacks.append(cb)


# Fake RPi.GPIO
sys.modules["RPi"] = types.ModuleType("RPi")
sys.modules["RPi.GPIO"] = GPIO

# ---------------- PIN LAYOUT ----------------

PIN_LAYOUT = {
    3: ("GPIO2", 2, "gpio"),
    5: ("GPIO3", 3, "gpio"),
    7: ("GPIO4", 4, "gpio"),
    8: ("GPIO14", 14, "gpio"),
    10: ("GPIO15", 15, "gpio"),
    11: ("GPIO17", 17, "gpio"),
    12: ("GPIO18", 18, "gpio"),
    13: ("GPIO27", 27, "gpio"),
    15: ("GPIO22", 22, "gpio"),
    16: ("GPIO23", 23, "gpio"),
    18: ("GPIO24", 24, "gpio"),
    19: ("GPIO10", 10, "gpio"),
    21: ("GPIO9", 9, "gpio"),
    22: ("GPIO25", 25, "gpio"),
    23: ("GPIO11", 11, "gpio"),
    24: ("GPIO8", 8, "gpio"),
    26: ("GPIO7", 7, "gpio"),
    27: ("GPIO0", 0, "gpio"),
    28: ("GPIO1", 1, "gpio"),
    29: ("GPIO5", 5, "gpio"),
    31: ("GPIO6", 6, "gpio"),
    32: ("GPIO12", 12, "gpio"),
    33: ("GPIO13", 13, "gpio"),
    35: ("GPIO19", 19, "gpio"),
    36: ("GPIO16", 16, "gpio"),
    37: ("GPIO26", 26, "gpio"),
    38: ("GPIO20", 20, "gpio"),
    40: ("GPIO21", 21, "gpio"),
}

bcm_to_label = {}
pin_counter = {bcm: 0 for _, bcm, _ in PIN_LAYOUT.values()}

# ---------------- GUI ----------------

root = tk.Tk()
root.title("Raspberry Pi Zero W – GPIO Dashboard")

main = tk.Frame(root)
main.pack(padx=10, pady=10)

left = tk.Frame(main)
right = tk.Frame(main)
left.grid(row=0, column=0, padx=5)
right.grid(row=0, column=1, padx=5)


def color_for(pin_type):
    return {"power": "#ffcccc", "gnd": "#cccccc", "gpio": "#aaaaaa"}[pin_type]


# ---------------- LOGGING ----------------

log_frame = tk.Frame(root)
log_frame.pack(pady=10, fill="both", expand=True)

tk.Label(log_frame, text="GPIO Log:").pack(anchor="w")

log_text = tk.Text(log_frame, height=10, state="disabled")
log_text.pack(fill="both", expand=True)

MAX_LOG_ENTRIES = 50
log_entries = []


def log(message):
    timestamp = datetime.now().strftime("%H:%M:%S")
    entry = f"[{timestamp}] {message}"
    log_entries.append(entry)
    if len(log_entries) > MAX_LOG_ENTRIES:
        log_entries.pop(0)
    log_text.config(state="normal")
    log_text.delete("1.0", "end")
    log_text.insert("end", "\n".join(log_entries) + "\n")
    log_text.see("end")
    log_text.config(state="disabled")


# ---------------- TIMELINE ----------------

timeline_frame = tk.Frame(root)
timeline_frame.pack(pady=5, fill="x")

timeline_canvas = tk.Canvas(timeline_frame, height=50, bg="#f0f0f0")
timeline_canvas.pack(fill="x", expand=True)

timeline_width = 800
timeline_canvas.config(width=timeline_width)
timeline_points = []


def add_timeline_event(bcm):
    x = len(timeline_points) * 10
    if x > timeline_width - 10:
        timeline_canvas.delete("all")
        timeline_points.clear()
        x = 0
    y = 25
    timeline_canvas.create_line(0, y, timeline_width, y, fill="#aaa")
    timeline_canvas.create_oval(
        x - 3, y - 3, x + 3, y + 3, fill="lime", outline="black"
    )
    timeline_points.append((x, bcm))


# ---------------- HEATMAP / BALKENDIAGRAMM ----------------

chart_frame = tk.Frame(root)
chart_frame.pack(pady=5, fill="both", expand=True)

tk.Label(chart_frame, text="Pin Statistik & Heatmap:").pack(anchor="w")
chart_canvas = tk.Canvas(chart_frame, height=200, bg="#f8f8f8")
chart_canvas.pack(fill="both", expand=True)


def update_chart():
    chart_canvas.delete("all")
    width = chart_canvas.winfo_width()
    height = chart_canvas.winfo_height()
    num_pins = len(pin_counter)
    if num_pins == 0:
        return
    max_count = max(pin_counter.values()) or 1
    bar_width = width / num_pins
    for i, bcm in enumerate(sorted(pin_counter.keys())):
        count = pin_counter[bcm]
        bar_height = (count / max_count) * (height - 20)
        x0 = i * bar_width
        y0 = height - bar_height
        x1 = (i + 1) * bar_width - 2
        y1 = height
        # Heatmap-Farbe: von hellblau (wenig) bis dunkelblau (viel)
        intensity = int(255 - (count / max_count) * 150)
        color = f"#00{intensity:02x}ff"
        chart_canvas.create_rectangle(x0, y0, x1, y1, fill=color)
        chart_canvas.create_text(
            x0 + bar_width / 2,
            height - 10,
            text=str(bcm),
            anchor="n",
            font=("Arial", 8),
        )
        # Pin-spezifische Statistik anzeigen
        chart_canvas.create_text(
            x0 + bar_width / 2,
            y0 - 10,
            text=str(count),
            anchor="s",
            font=("Arial", 8, "bold"),
        )


# ---------------- GESAMTSTATISTIK ----------------

stats_frame = tk.Frame(root)
stats_frame.pack(pady=5, fill="x")
stats_label = tk.Label(stats_frame, text="Gesamtstatistiken:")
stats_label.pack(anchor="w")


def update_stats():
    total_switches = sum(pin_counter.values())
    high_pins = sum(1 for bcm in bcm_to_label if GPIO.input(bcm))
    stats_label.config(
        text=f"Gesamt-Schaltungen: {total_switches} | Aktuell HIGH-Pins: {high_pins}"
    )
    root.after(1000, update_stats)


# ---------------- GPIO UPDATE & CLICK ----------------


def update_gpio(bcm, state):
    if bcm in bcm_to_label:
        # Heatmap: je häufiger der Pin geschaltet wurde, desto intensiver grün
        count = pin_counter[bcm]
        intensity = min(255, 50 + count * 10)
        color = f"#00{intensity:02x}00" if state else color_for("gpio")
        bcm_to_label[bcm].config(bg=color)


def update_gpio_and_log(bcm, state):
    update_gpio(bcm, state)
    log(f"GPIO {bcm} -> {'HIGH' if state else 'LOW'}")
    add_timeline_event(bcm)
    pin_counter[bcm] += 1
    update_chart()


register_callback(update_gpio_and_log)


def toggle_pin(event, bcm):
    current = GPIO.input(bcm)
    GPIO.output(bcm, not current)


# ---------------- CREATE PIN LABELS ----------------

for phys, (text, bcm, ptype) in PIN_LAYOUT.items():
    frame = left if phys % 2 == 1 else right
    lbl = tk.Label(
        frame, text=f"{phys:>2} {text}", width=14, bg=color_for(ptype), relief="raised"
    )
    lbl.grid(row=(phys - 1) // 2, column=0, pady=1)
    if ptype == "gpio":
        bcm_to_label[bcm] = lbl
        lbl.bind("<Button-1>", lambda e, b=bcm: toggle_pin(e, b))

# ---------------- PROGRAMM START ----------------

controls = tk.Frame(root)
controls.pack(pady=10)

tk.Label(controls, text="Python-Programm:").grid(row=0, column=0)
entry_file = tk.Entry(controls, width=35)
entry_file.insert(0, "hw-controller/main.py")
entry_file.grid(row=0, column=1)

tk.Label(controls, text="Argumente:").grid(row=1, column=0)
entry_args = tk.Entry(controls, width=35)
entry_args.insert(0, "--sx 100 --sy 10 --sz 10 --ex -100 --ey 500 --ez 100 --time 1")
entry_args.grid(row=1, column=1)

program_thread = None
stop_flag = False


def start_program():
    global program_thread, stop_flag
    stop_flag = False

    def runner():
        sys.argv = [entry_file.get()] + entry_args.get().split()
        local_vars = {"__name__": "__main__", "stop_flag": stop_flag}
        with open(entry_file.get(), "r", encoding="utf-8") as f:
            exec(f.read(), local_vars)

    program_thread = threading.Thread(target=runner, daemon=True)
    program_thread.start()


def stop_program():
    global stop_flag
    stop_flag = True
    log("Programm gestoppt.")


def reset_all_pins():
    for bcm in bcm_to_label.keys():
        GPIO.output(bcm, False)
        pin_counter[bcm] = 0
    log("Alle Pins auf LOW gesetzt und Zähler zurückgesetzt.")
    update_chart()


# ---------------- FILTER ----------------


def filter_pins():
    query = filter_entry.get().strip()
    for bcm, lbl in bcm_to_label.items():
        state = GPIO.input(bcm)
        show = True
        if query.lower() == "high" and not state:
            show = False
        elif query.lower() == "low" and state:
            show = False
        elif query.isdigit() and int(query) != bcm:
            show = False
        lbl.grid() if show else lbl.grid_remove()


# ---------------- CONTROLS BUTTONS ----------------

tk.Button(controls, text="Start", command=start_program).grid(row=2, column=0, pady=5)
tk.Button(controls, text="Stop", command=stop_program).grid(row=2, column=1, pady=5)
tk.Button(controls, text="Reset", command=reset_all_pins).grid(row=2, column=2, pady=5)

tk.Label(controls, text="Filter:").grid(row=3, column=0)
filter_entry = tk.Entry(controls, width=20)
filter_entry.grid(row=3, column=1, sticky="w")
tk.Button(controls, text="Anwenden", command=filter_pins).grid(row=3, column=2)

# ---------------- START UPDATES ----------------

update_stats()
root.mainloop()
