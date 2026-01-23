const express = require('express');
const path = require('path');
const fs = require('fs');
const cors = require('cors');

const app = express();
const DATA_FILE = path.join(__dirname, 'data.json');

app.use(cors());
app.use(express.json());
app.use(express.static(path.join(__dirname, 'public')));

function readData() {
  try {
    const raw = fs.readFileSync(DATA_FILE, 'utf8');
    return JSON.parse(raw || '[]');
  } catch (e) {
    return [];
  }
}

function writeData(data) {
  fs.writeFileSync(DATA_FILE, JSON.stringify(data, null, 2));
}

app.get('/api/infos', (req, res) => {
  const data = readData();
  res.json(data);
});

app.post('/api/infos', (req, res) => {
  const { title, body } = req.body;
  if (!title) return res.status(400).json({ error: 'title required' });
  const data = readData();
  const item = {
    id: Date.now(),
    title,
    body: body || '',
    createdAt: new Date().toISOString(),
  };
  data.unshift(item);
  writeData(data);
  res.status(201).json(item);
});

app.get('*', (req, res) => {
  res.sendFile(path.join(__dirname, 'public', 'index.html'));
});

const PORT = process.env.PORT || 3000;
app.listen(PORT, () => console.log(`Server listening on ${PORT}`));
