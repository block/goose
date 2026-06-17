import fsSync from 'node:fs';
import { execFile as nodeExecFile } from 'node:child_process';

interface OpenDirectoryInExplorerDeps {
  directoryExists: (directoryPath: string) => boolean;
  execFile: (
    command: string,
    args: string[],
    callback: (error: Error | null) => void
  ) => void;
  openPath: (directoryPath: string) => Promise<string>;
  platform: string;
}

export interface OpenDirectoryInExplorerResult {
  error?: string;
  opened: boolean;
}

function defaultDirectoryExists(directoryPath: string): boolean {
  try {
    return fsSync.statSync(directoryPath).isDirectory();
  } catch {
    return false;
  }
}

const defaultDeps: OpenDirectoryInExplorerDeps = {
  directoryExists: defaultDirectoryExists,
  execFile: (command, args, callback) => {
    nodeExecFile(command, args, (error) => callback(error));
  },
  openPath: async () => {
    throw new Error('openPath dependency not provided');
  },
  platform: process.platform,
};

const MACOS_OPEN_BINARY = '/usr/bin/open';

export async function openDirectoryInExplorer(
  directoryPath: string,
  deps: Partial<OpenDirectoryInExplorerDeps> = {}
): Promise<OpenDirectoryInExplorerResult> {
  const resolvedDeps = {
    ...defaultDeps,
    ...deps,
  };

  if (!directoryPath || !resolvedDeps.directoryExists(directoryPath)) {
    return {
      error: 'Directory does not exist.',
      opened: false,
    };
  }

  if (resolvedDeps.platform === 'darwin') {
    const commandResult = await new Promise<OpenDirectoryInExplorerResult>((resolve) => {
      resolvedDeps.execFile(MACOS_OPEN_BINARY, ['-R', directoryPath], (error) =>
        resolve({
          error: error?.message,
          opened: error === null,
        })
      );
    });

    if (commandResult.opened) {
      return commandResult;
    }

    const fallbackResult = await resolvedDeps.openPath(directoryPath);
    return fallbackResult === ''
      ? { opened: true }
      : {
          error: commandResult.error || fallbackResult,
          opened: false,
        };
  }

  const result = await resolvedDeps.openPath(directoryPath);
  return result === ''
    ? { opened: true }
    : {
        error: result,
        opened: false,
      };
}
