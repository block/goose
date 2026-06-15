#!/usr/bin/env node

const { execFileSync } = require('node:child_process');
const path = require('node:path');

function parseProcessTable(processTable) {
  return processTable
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => {
      const match = line.match(/^(\d+)\s+(.*)$/);
      if (!match) {
        return null;
      }

      return {
        pid: Number(match[1]),
        command: match[2],
      };
    })
    .filter((entry) => entry !== null);
}

function selectPreviewElectronPid(processTable, electronBinary, repoRoot) {
  const repoMarker = `--dir ${repoRoot}`;
  const candidates = parseProcessTable(processTable).filter(
    (entry) =>
      entry.command.includes(electronBinary) &&
      entry.command.includes(repoMarker) &&
      !entry.command.includes('--type=')
  );

  return candidates.length > 0 ? candidates[candidates.length - 1].pid : null;
}

function buildAppleScriptForPid(pid) {
  return [
    'tell application "System Events"',
    `  set targetProcess to first application process whose unix id is ${pid}`,
    '  set frontmost of targetProcess to true',
    '  tell targetProcess',
    '    try',
    '      perform action "AXRaise" of window 1',
    '    end try',
    '  end tell',
    'end tell',
  ].join('\n');
}

function main() {
  const repoRoot = process.env.GOOSE_PREVIEW_REPO_ROOT?.trim() || path.resolve(__dirname, '..');
  const electronBinary = path.join(
    repoRoot,
    'ui',
    'node_modules',
    'electron',
    'dist',
    'Electron.app',
    'Contents',
    'MacOS',
    'Electron'
  );

  let processTable = '';
  try {
    processTable = execFileSync('ps', ['-Ao', 'pid=,command='], {
      encoding: 'utf8',
    });
  } catch {
    process.exit(0);
  }

  const pid = selectPreviewElectronPid(processTable, electronBinary, repoRoot);
  if (!pid) {
    process.exit(0);
  }

  try {
    execFileSync('osascript', ['-e', buildAppleScriptForPid(pid)], {
      stdio: 'ignore',
    });
  } catch {
    process.exit(0);
  }
}

if (require.main === module) {
  main();
}

module.exports = {
  buildAppleScriptForPid,
  main,
  parseProcessTable,
  selectPreviewElectronPid,
};
