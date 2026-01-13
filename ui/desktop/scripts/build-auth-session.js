const { spawnSync } = require('child_process');
const fs = require('fs');
const path = require('path');

if (process.platform !== 'darwin') {
  console.log('Skipping auth session native build (macOS only).');
  process.exit(0);
}

const addonDir = path.join(__dirname, '..', 'src', 'native', 'auth_session');
if (!fs.existsSync(addonDir)) {
  console.warn(`Auth session addon directory not found: ${addonDir}`);
  process.exit(0);
}

let nodeGypPath = null;
try {
  nodeGypPath = require.resolve('@electron/node-gyp/bin/node-gyp.js');
} catch {
  console.error(
    'Missing @electron/node-gyp. Install dependencies before building the auth session addon.'
  );
  process.exit(1);
}

const command = process.execPath;
const args = [nodeGypPath, 'rebuild'];

console.log('Building auth session native addon...');
const result = spawnSync(command, args, {
  cwd: addonDir,
  stdio: 'inherit',
  env: process.env,
});

if (result.error) {
  console.error('Failed to build auth session addon:', result.error);
  process.exit(1);
}

if (result.status !== 0) {
  process.exit(result.status ?? 1);
}
