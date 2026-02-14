import math as m
import tkinter as tk
from tkinter import ttk
import matplotlib.pyplot as plt
from matplotlib.backends.backend_tkagg import FigureCanvasTkAgg
import numpy as np

# Roboter Parameter
length_arm_1_mm = 500
length_arm_2_mm = 525

# =========================
# Utility Funktionen
# =========================

def inverse_kinematics(x, z):
    r = m.sqrt(x**2 + z**2)
    if r > (length_arm_1_mm + length_arm_2_mm):
        return None, None
    alpha = m.atan2(z, x)
    cos_theta2 = (r**2 - length_arm_1_mm**2 - length_arm_2_mm**2) / (2 * length_arm_1_mm * length_arm_2_mm)
    cos_theta2 = max(-1, min(1, cos_theta2))
    theta2 = m.acos(cos_theta2)
    theta1 = alpha - m.atan2(length_arm_2_mm * m.sin(theta2),
                             length_arm_1_mm + length_arm_2_mm * m.cos(theta2))
    return theta1, theta2

def check_bounds(x, z):
    r = m.sqrt(x**2 + z**2)
    return r <= (length_arm_1_mm + length_arm_2_mm)

# =========================
# Hauptklasse
# =========================

class RobotSimulatorPreview:
    def __init__(self, master):
        self.master = master
        master.title("Roboter Simulation mit Vorschau")
        master.geometry("900x700")

        # 3D Plot
        self.fig = plt.figure(figsize=(6,6), facecolor="#e0f0ff")
        self.ax = self.fig.add_subplot(111, projection='3d')
        self.ax.set_box_aspect([1,1,1])
        self.canvas = FigureCanvasTkAgg(self.fig, master=master)
        self.canvas.get_tk_widget().pack(side=tk.RIGHT, fill=tk.BOTH, expand=True)

        # Steuerung
        ctrl_frame = ttk.Frame(master)
        ctrl_frame.pack(side=tk.LEFT, fill=tk.Y, padx=10, pady=10)

        ttk.Label(ctrl_frame, text="Start X [mm]").grid(row=0, column=0)
        self.start_x = ttk.Entry(ctrl_frame)
        self.start_x.grid(row=0, column=1)
        self.start_x.insert(0, "300")

        ttk.Label(ctrl_frame, text="Start Z [mm]").grid(row=1, column=0)
        self.start_z = ttk.Entry(ctrl_frame)
        self.start_z.grid(row=1, column=1)
        self.start_z.insert(0, "300")

        ttk.Label(ctrl_frame, text="End X [mm]").grid(row=2, column=0)
        self.end_x = ttk.Entry(ctrl_frame)
        self.end_x.grid(row=2, column=1)
        self.end_x.insert(0, "600")

        ttk.Label(ctrl_frame, text="End Z [mm]").grid(row=3, column=0)
        self.end_z = ttk.Entry(ctrl_frame)
        self.end_z.grid(row=3, column=1)
        self.end_z.insert(0, "400")

        ttk.Button(ctrl_frame, text="Animation starten", command=self.start_animation).grid(row=4, column=0, columnspan=2, pady=10)

        # Vorschau
        self.update_preview()
        self.start_x.bind("<KeyRelease>", lambda e: self.update_preview())
        self.start_z.bind("<KeyRelease>", lambda e: self.update_preview())
        self.end_x.bind("<KeyRelease>", lambda e: self.update_preview())
        self.end_z.bind("<KeyRelease>", lambda e: self.update_preview())

    # =========================
    # Vorschau aktualisieren
    # =========================
    def update_preview(self):
        try:
            sx = float(self.start_x.get())
            sz = float(self.start_z.get())
            ex = float(self.end_x.get())
            ez = float(self.end_z.get())

            x_vals = np.linspace(sx, ex, 50)
            z_vals = np.linspace(sz, ez, 50)

            self.ax.clear()
            self.ax.plot(x_vals, np.zeros_like(x_vals), z_vals, '-o', color='blue', alpha=0.5, label="Pfad Vorschau")
            self.ax.scatter([ex], [0], [ez], color='red', s=80, label="Endposition")
            self.ax.set_xlabel("X [mm]")
            self.ax.set_ylabel("Y [°]")
            self.ax.set_zlabel("Z [mm]")
            self.ax.set_title("Endeffektor Vorschau")
            self.ax.set_box_aspect([1,1,1])
            self.ax.legend()
            self.canvas.draw()
        except:
            pass  # noch keine gültigen Zahlen

    # =========================
    # Animation
    # =========================
    def start_animation(self):
        try:
            sx = float(self.start_x.get())
            sz = float(self.start_z.get())
            ex = float(self.end_x.get())
            ez = float(self.end_z.get())

            self.x_vals = np.linspace(sx, ex, 50)
            self.z_vals = np.linspace(sz, ez, 50)
            self.anim_index = 0

            self.anim_point, = self.ax.plot([], [], [], 'ro', markersize=8, label="Spitze")
            self.anim_path, = self.ax.plot([], [], [], color='blue', alpha=0.4, label="Bahn")
            self.animate_step()
        except Exception as e:
            print("Fehler:", e)

    def animate_step(self):
        if self.anim_index < len(self.x_vals):
            x = self.x_vals[self.anim_index]
            z = self.z_vals[self.anim_index]
            self.anim_point.set_data([x], [0])
            self.anim_point.set_3d_properties([z])
            self.anim_path.set_data(self.x_vals[:self.anim_index+1], np.zeros(self.anim_index+1))
            self.anim_path.set_3d_properties(self.z_vals[:self.anim_index+1])
            self.canvas.draw()
            self.anim_index += 1
            self.master.after(30, self.animate_step)  # flüssige Animation
        else:
            print("Animation fertig")

# =========================
# Start GUI
# =========================
root = tk.Tk()
app = RobotSimulatorPreview(root)
root.mainloop()
