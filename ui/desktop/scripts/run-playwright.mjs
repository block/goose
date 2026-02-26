#!/usr/bin/env node
import { spawnSync } from 'node:child_process';

function hasCommand(cmd) {
  const result = spawnSync('bash', ['-lc', `command -v ${cmd}`], { stdio: 'ignore' });
  return result.status === 0;
}

function isHeadlessLinux() {
  if (process.platform !== 'linux') {
    return false;
  }

  // Electron can use X11 (DISPLAY) or Wayland (WAYLAND_DISPLAY).
  return !process.env.DISPLAY && !process.env.WAYLAND_DISPLAY;
}

const args = process.argv.slice(2);
const playwrightArgs = args.length > 0 ? args : ['test'];

if (isHeadlessLinux()) {
  if (hasCommand('xvfb-run')) {
    const cmd = ['xvfb-run', '-a', 'playwright', ...playwrightArgs];
    const result = spawnSync(cmd[0], cmd.slice(1), { stdio: 'inherit', env: process.env });
    process.exit(result.status ?? 1);
  }

  console.error(
    [
      'Playwright E2E requires a display server on Linux.',
      'No DISPLAY/WAYLAND_DISPLAY detected, and xvfb-run is not installed.',
      '',
      'Fix options:',
      '  1) Install Xvfb and re-run (recommended for CI):',
      '       sudo apt-get update && sudo apt-get install -y xvfb',
      '  2) Run in a desktop session (set DISPLAY or WAYLAND_DISPLAY).',
      '',
      'Then re-run the command.',
    ].join('\n')
  );
  process.exit(1);
}

const result = spawnSync('playwright', playwrightArgs, { stdio: 'inherit', env: process.env });
process.exit(result.status ?? 1);
