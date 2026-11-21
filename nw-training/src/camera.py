
import argparse
import cv2
from ultralytics import YOLO


def run_camera(weights, camera_id, confidence, device):
    model = YOLO(weights)

    camera = cv2.VideoCapture(camera_id)

    if not camera.isOpened():
        raise RuntimeError("Could not open camera!")

    print("Camera started. Press Q to quit.")

    try:
        while True:
            success, frame = camera.read()

            if not success:
                break

            results = model.predict(
                source=frame,
                conf=confidence,
                imgsz=640,
                device=device,
                verbose=False
            )

            output = results[0].plot()

            cv2.imshow("YOLO Object Detection", output)

            if cv2.waitKey(1) & 0xFF in (ord("q"), 27):
                break

    finally:
        camera.release()
        cv2.destroyAllWindows()


if __name__ == "__main__":
    parser = argparse.ArgumentParser()

    parser.add_argument(
        "--weights",
        default="models/best.pt"
    )

    parser.add_argument("--camera", type=int, default=0)
    parser.add_argument("--conf", type=float, default=0.35)
    parser.add_argument("--device", default="cpu")

    args = parser.parse_args()

    run_camera(
        args.weights,
        args.camera,
        args.conf,
        args.device
    )
