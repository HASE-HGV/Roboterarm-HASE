
import torch
import torch.nn as nn
import torch.optim as optim

from model import CNN
from dataset import get_dataloaders


def train():
    device = torch.device(
        "cuda" if torch.cuda.is_available() else "cpu"
    )

    print("Device:", device)

    train_loader, val_loader = get_dataloaders()

    model = CNN().to(device)

    criterion = nn.CrossEntropyLoss()
    optimizer = optim.Adam(
        model.parameters(),
        lr=0.001
    )

    epochs = 10

    for epoch in range(epochs):
        model.train()

        total_loss = 0

        for images, labels in train_loader:
            images = images.to(device)
            labels = labels.to(device)

            optimizer.zero_grad()

            outputs = model(images)
            loss = criterion(outputs, labels)

            loss.backward()
            optimizer.step()

            total_loss += loss.item()

        model.eval()

        correct = 0
        total = 0

        with torch.no_grad():
            for images, labels in val_loader:
                images = images.to(device)
                labels = labels.to(device)

                outputs = model(images)
                predictions = outputs.argmax(dim=1)

                correct += (predictions == labels).sum().item()
                total += labels.size(0)

        accuracy = 100 * correct / total
        avg_loss = total_loss / len(train_loader)

        print("Epoch",epoch + 1/epochs, "Loss", avg_loss, "Accuracy", accuracy)

    torch.save({
        "model_state": model.state_dict(),
        "classes": train_loader.dataset.classes
    }, "models/classifier.pth")

    print("Model saved!")


if __name__ == "__main__":
    import os
    os.makedirs("models", exist_ok=True)
    train()
