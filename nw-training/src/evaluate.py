
import argparse
from ultralytics import YOLO


def evaluate(weights, device):
    model = YOLO(weights)

    metrics = model.val(
        data="coco.yaml",
        imgsz=640,
        batch=16,
        device=device,
        plots=True
    )

    print("\nEvaluation results")
    print("------------------")
    print(f"Precision: {metrics.box.mp:.4f}")
    print(f"Recall:    {metrics.box.mr:.4f}")
    print(f"mAP50:     {metrics.box.map50:.4f}")
    print(f"mAP50-95:  {metrics.box.map:.4f}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()

    parser.add_argument(
        "--weights",
        default="runs/yolo-scratch/weights/best.pt"
    )

    parser.add_argument("--device", default="0")

    args = parser.parse_args()

    evaluate(args.weights, args.device)
