
import argparse
import torch
from pathlib import Path
from ultralytics import YOLO


def train(args):
    print("PyTorch:", torch.__version__)
    print("CUDA:", torch.cuda.is_available())

    if torch.cuda.is_available():
        print("GPU:", torch.cuda.get_device_name(0))


    model = YOLO("yolo11n.yaml")

    print("\nStarting training from scratch...")

    model.train(
        data="coco.yaml",
        epochs=args.epochs,
        imgsz=args.imgsz,
        batch=args.batch,
        workers=args.workers,
        device=args.device,
        pretrained=False,
        optimizer="AdamW",
        lr0=0.001,
        weight_decay=0.0005,
        patience=args.patience,
        project=args.project,
        name=args.name,
        exist_ok=True,
        amp=True,
        plots=True,
        save=True,
        fraction=args.fraction
    )

    best = Path(args.project) / args.name / "weights" / "best.pt"

    print("\nTraining finished.")
    print("Best model:", best)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()

    parser.add_argument("--epochs", type=int, default=100)
    parser.add_argument("--imgsz", type=int, default=640)
    parser.add_argument("--batch", type=int, default=16)
    parser.add_argument("--workers", type=int, default=4)
    parser.add_argument("--device", default="0")
    parser.add_argument("--patience", type=int, default=30)
    parser.add_argument("--fraction", type=float, default=1.0)
    parser.add_argument("--project", default="runs")
    parser.add_argument("--name", default="yolo-scratch")

    train(parser.parse_args())
