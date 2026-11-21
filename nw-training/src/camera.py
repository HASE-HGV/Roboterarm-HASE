
import cv2
import torch
import numpy as np

from PIL import Image
from torchvision import transforms

from model import CNN


device = torch.device(
    "cuda" if torch.cuda.is_available() else "cpu"
)

checkpoint = torch.load(
    "models/classifier.pth",
    map_location=device,
    weights_only=True
)

classes = checkpoint["classes"]

model = CNN(num_classes=len(classes)).to(device)
model.load_state_dict(checkpoint["model_state"])
model.eval()

transform = transforms.Compose([
    transforms.Resize((32, 32)),
    transforms.ToTensor(),
    transforms.Normalize(
        (0.4914, 0.4822, 0.4465),
        (0.247, 0.243, 0.261)
    )
])

camera = cv2.VideoCapture(0)

if not camera.isOpened():
    raise RuntimeError("Camera not found!")

while True:
    ret, frame = camera.read()

    if not ret:
        break

    rgb = cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
    image = Image.fromarray(rgb)

    tensor = transform(image).unsqueeze(0).to(device)

    with torch.no_grad():
        outputs = model(tensor)
        probabilities = torch.softmax(outputs, dim=1)

        confidence, prediction = probabilities.max(dim=1)

    label = classes[prediction.item()]
    score = confidence.item() * 100

    cv2.putText(
        frame,
        f"{label}: {score:.1f}%",
        (20, 40),
        cv2.FONT_HERSHEY_SIMPLEX,
        1,
        (0, 255, 0),
        2
    )

    cv2.imshow("Neural Network Camera", frame)

    if cv2.waitKey(1) & 0xFF == ord("q"):
        break

camera.release()
cv2.destroyAllWindows()
