import { app } from 'electron';
import * as path from 'path';
import { spawn as spawnProcess } from 'child_process';
import * as fs from 'fs';

export function handleSquirrelEvent(): boolean {
  if (process.argv.length === 1) {
    return false;
  }

  const appFolder = path.resolve(process.execPath, '..');
  const rootAtomFolder = path.resolve(appFolder, '..');
  const updateDotExe = path.resolve(path.join(rootAtomFolder, 'Update.exe'));
  const exeName = path.basename(process.execPath);

  const spawnUpdate = function (args: string[]) {
    try {
      return spawnProcess(updateDotExe, args, { detached: true });
    } catch (error) {
      console.error('Failed to spawn update process:', error);
      return null;
    }
  };

  const squirrelEvent = process.argv[1];
  switch (squirrelEvent) {
    case '--squirrel-install':
    case '--squirrel-updated': {
      // Register protocol handler
      spawnUpdate(['--createShortcut', exeName]);

      // Register protocol
      const regCommand = `Windows Registry Editor Version 5.00

[HKEY_CLASSES_ROOT\\goose]
@="URL:Goose Protocol"
"URL Protocol"=""

[HKEY_CLASSES_ROOT\\goose\\DefaultIcon]
@="\\"${process.execPath.replace(/\\/g, '\\\\')},1\\""

[HKEY_CLASSES_ROOT\\goose\\shell]

[HKEY_CLASSES_ROOT\\goose\\shell\\open]

[HKEY_CLASSES_ROOT\\goose\\shell\\open\\command]
@="\\"${process.execPath.replace(/\\/g, '\\\\')}\\" \\"%1\\""`;

      fs.writeFileSync('goose-protocol.reg', regCommand);
      spawnProcess('regedit.exe', ['/s', 'goose-protocol.reg']);

      setTimeout(() => app.quit(), 1000);
      return true;
    }

    case '--squirrel-uninstall': {
      // Remove protocol handler
      spawnUpdate(['--removeShortcut', exeName]);
      setTimeout(() => app.quit(), 1000);
      return true;
    }

    case '--squirrel-obsolete': {
      app.quit();
      return true;
    }

    default:
      return false;
  }
}
