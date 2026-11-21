
import numpy as np


class NeuralNetwork:
    def __init__(self, input_size, hidden_size, output_size):
        self.W1 = np.random.randn(input_size, hidden_size) * 0.01
        self.b1 = np.zeros((1, hidden_size))

        self.W2 = np.random.randn(hidden_size, output_size) * 0.01
        self.b2 = np.zeros((1, output_size))

    def forward(self, X):
        self.z1 = X @ self.W1 + self.b1
        self.a1 = np.maximum(0, self.z1)

        self.z2 = self.a1 @ self.W2 + self.b2

        exp = np.exp(self.z2 - np.max(self.z2, axis=1, keepdims=True))
        self.output = exp / np.sum(exp, axis=1, keepdims=True)

        return self.output

    def backward(self, X, y, lr):
        n = X.shape[0]

        dz2 = self.output.copy()
        dz2[np.arange(n), y] -= 1
        dz2 /= n

        dW2 = self.a1.T @ dz2
        db2 = np.sum(dz2, axis=0, keepdims=True)

        dz1 = (dz2 @ self.W2.T) * (self.z1 > 0)

        dW1 = X.T @ dz1
        db1 = np.sum(dz1, axis=0, keepdims=True)

        self.W2 -= lr * dW2
        self.b2 -= lr * db2

        self.W1 -= lr * dW1
        self.b1 -= lr * db1

    def train(self, X, y, epochs=1000, lr=0.1):
        for epoch in range(epochs):
            predictions = self.forward(X)

            loss = -np.mean(
                np.log(predictions[np.arange(len(y)), y] + 1e-9)
            )

            self.backward(X, y, lr)

            if epoch % 100 == 0:
                print(f"Epoch {epoch}: Loss {loss:.4f}")

    def predict(self, X):
        return np.argmax(self.forward(X), axis=1)


X = np.array([
    [0, 0],
    [2, 2],
    [0, 1],
    [1, 0],
    [1, 1]
])

y = np.array([0,0, 1, 1, 0])

nn = NeuralNetwork(
    input_size=2,
    hidden_size=10,
    output_size=2
)

nn.train(X, y, epochs=5000, lr=0.1)

print("Predictions:", nn.predict(X))
