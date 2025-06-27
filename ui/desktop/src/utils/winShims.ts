import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { spawn } from 'child_process';
import log from './logger';

/**
 * Ensures Windows shims are available in %LOCALAPPDATA%\Goose\bin
 * This allows the bundled executables to be found via PATH regardless of where Goose is installed
 */
export async function ensureWinShims(): Promise<void> {
  if (process.platform !== 'win32') return;

  const srcDir = path.join(process.resourcesPath, 'bin'); // existing dir
  const tgtDir = path.join(
    process.env.LOCALAPPDATA ?? path.join(os.homedir(), 'AppData', 'Local'),
    'Goose',
    'bin'
  );

  try {
    await fs.promises.mkdir(tgtDir, { recursive: true });

    // Only copy the command-line tools, NOT goosed.exe (which should always be used locally)
    const shims = ['uvx.exe', 'npx.cmd', 'install-node.cmd'];

    await Promise.all(
      shims.map(async (shim) => {
        const src = path.join(srcDir, shim);
        const dst = path.join(tgtDir, shim);
        try {
          // Check if source file exists before attempting to copy
          await fs.promises.access(src);
          await fs.promises.copyFile(src, dst); // overwrites with newer build
          log.info(`Copied Windows shim: ${shim} to ${dst}`);
        } catch (e) {
          log.error(`Failed to copy shim ${shim}`, e);
        }
      })
    );

    // Prepend to PATH **for this process & all children**.
    // Make sure our bin directory is at the VERY BEGINNING of PATH
    const currentPath = process.env.PATH ?? '';
    if (!currentPath.toLowerCase().includes(tgtDir.toLowerCase())) {
      process.env.PATH = `${tgtDir}${path.delimiter}${currentPath}`;
      log.info(`Added ${tgtDir} to PATH for current process`);
    } else {
      // If it's already in PATH, make sure it's at the beginning
      const pathParts = currentPath.split(path.delimiter);
      const binDirIndex = pathParts.findIndex((p) => p.toLowerCase() === tgtDir.toLowerCase());

      if (binDirIndex > 0) {
        // Remove it from its current position and add to beginning
        pathParts.splice(binDirIndex, 1);
        process.env.PATH = `${tgtDir}${path.delimiter}${pathParts.join(path.delimiter)}`;
        log.info(`Moved ${tgtDir} to beginning of PATH for current process`);
      }
    }

    // Optional: Persist PATH for user's external PowerShell/CMD sessions
    await persistPathForUser(tgtDir);
  } catch (error) {
    log.error('Failed to ensure Windows shims:', error);
  }
}

/**
 * Optionally persist the Goose bin directory to the user's PATH environment variable
 * This allows users to run uvx, npx, goosed from external PowerShell/CMD sessions
 */
async function persistPathForUser(binDir: string): Promise<void> {
  try {
    const psScript = `
      $bin = "${binDir.replace(/\\/g, '\\\\')}"
      if (-not ($Env:Path -split ';' | Where-Object { $_ -ieq $bin })) {
        # Add to beginning of PATH for priority
        setx PATH "$bin;$Env:Path" >$null
        Write-Host "Added Goose bin directory to beginning of user PATH"
      } else {
        # If already in PATH, ensure it's at the beginning
        $pathParts = $Env:Path -split ';'
        $binIndex = 0
        for ($i = 0; $i -lt $pathParts.Count; $i++) {
          if ($pathParts[$i] -ieq $bin) {
            $binIndex = $i
            break
          }
        }
        
        if ($binIndex -gt 0) {
          # Remove from current position and add to beginning
          $pathParts = @($pathParts[$binIndex]) + @($pathParts | Where-Object { $_ -ine $bin })
          $newPath = $pathParts -join ';'
          setx PATH $newPath >$null
          Write-Host "Moved Goose bin directory to beginning of user PATH"
        } else {
          Write-Host "Goose bin directory already at beginning of user PATH"
        }
      }
    `;

    spawn('powershell', ['-NoProfile', '-NonInteractive', '-Command', psScript], {
      windowsHide: true,
      shell: false,
    });

    log.info('Attempted to persist Goose bin directory to user PATH');
  } catch (error) {
    log.warn('Failed to persist PATH for user (non-critical):', error);
  }
}
