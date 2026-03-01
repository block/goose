#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

function hasCommand(cmd) {
  const result = spawnSync('bash', ['-lc', `command -v ${cmd}`], { stdio: 'ignore' });
  return result.status === 0;
}

const FORCE_XVFB_LINUX = process.platform === 'linux';

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

if (FORCE_XVFB_LINUX) {
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
      'Playwright E2E on Linux requires Xvfb (xvfb-run).',
      'Install it and re-run:',
      '  sudo dnf install -y xorg-x11-server-Xvfb',
    ].join('\n')
  );

  process.exit(1);
}

process.exit(runPlaywright(cwd, playwrightArgs));
