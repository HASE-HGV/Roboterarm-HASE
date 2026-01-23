const fs = require('fs');
const path = require('path');

const src = path.join(__dirname, 'public');
const out = path.join(__dirname, 'build');

function copyDir(srcDir, outDir) {
  if (!fs.existsSync(outDir)) fs.mkdirSync(outDir, { recursive: true });
  for (const name of fs.readdirSync(srcDir)) {
    const from = path.join(srcDir, name);
    const to = path.join(outDir, name);
    const stat = fs.statSync(from);
    if (stat.isDirectory()) {
      copyDir(from, to);
    } else {
      fs.copyFileSync(from, to);
    }
  }
}

try {
  if (fs.existsSync(out)) fs.rmSync(out, { recursive: true, force: true });
  copyDir(src, out);
  // copy server.js so the build can still be served by node if desired
  fs.copyFileSync(path.join(__dirname, 'server.js'), path.join(out, 'server.js'));
  console.log('Build created in', out);
} catch (err) {
  console.error('Build failed:', err);
  process.exit(1);
}
