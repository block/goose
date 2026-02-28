#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

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

function resolvePlaywrightInvocation(cwd) {
  // Prefer the project-local Playwright CLI (works even when PATH doesn't include node_modules/.bin)
  const localCli = path.join(cwd, 'node_modules', '@playwright', 'test', 'cli.js');
  if (fs.existsSync(localCli)) {
    return { cmd: process.execPath, argsPrefix: [localCli] };
  }

  return { cmd: 'playwright', argsPrefix: [] };
}

function runPlaywright(cwd, playwrightArgs) {
  const { cmd, argsPrefix } = resolvePlaywrightInvocation(cwd);

  const result = spawnSync(cmd, [...argsPrefix, ...playwrightArgs], {
    stdio: 'inherit',
    env: process.env,
    cwd,
  });

  if (result.error?.code === 'ENOENT') {
    console.error(
      [
        `Failed to run Playwright CLI: ${cmd} not found.`,
        '',
        'Expected one of:',
        '  - a local dependency at node_modules/@playwright/test',
        '  - a global `playwright` binary on PATH',
        '',
        'Fix:',
        '  npm install',
      ].join('\n')
    );
  }

  return result.status ?? 1;
}

const cwd = process.cwd();
const args = process.argv.slice(2);
const playwrightArgs = args.length > 0 ? args : ['test'];

if (isHeadlessLinux()) {
  if (hasCommand('xvfb-run')) {
    const { cmd, argsPrefix } = resolvePlaywrightInvocation(cwd);

    const result = spawnSync('xvfb-run', ['-a', cmd, ...argsPrefix, ...playwrightArgs], {
      stdio: 'inherit',
      env: process.env,
      cwd,
    });

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

process.exit(runPlaywright(cwd, playwrightArgs));
