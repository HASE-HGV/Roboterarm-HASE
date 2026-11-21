
import subprocess
import sys
import os


subprocess.check_call([
    sys.executable,
    "-m", "pip", "install", "-q",
    "ultralytics"
])


subprocess.check_call([
    sys.executable,
    "src/train.py",
    "--epochs", "100",
    "--imgsz", "640",
    "--batch", "16",
    "--workers", "2",
    "--device", "0",
    "--project", "/kaggle/working/runs",
    "--name", "yolo-scratch",
])

print("Training completed!")

print(
    "Model:",
    "/kaggle/working/runs/yolo-scratch/weights/best.pt"
)
