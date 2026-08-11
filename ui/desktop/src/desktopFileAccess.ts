import fs from 'node:fs/promises';
import { constants as fsConstants } from 'node:fs';
import path from 'node:path';

export interface FileReadResult {
  file: string;
  filePath: string;
  error: string | null;
  found: boolean;
}

interface FileAccessRequestProvenance {
  isRegisteredWindow: boolean;
  isMainFrame: boolean;
  rendererUrl: string;
}

export function isAppRendererUrl(rendererUrl: string, expectedUrl: URL): boolean {
  try {
    const actual = new URL(rendererUrl);
    if (expectedUrl.protocol === 'file:') {
      return (
        actual.protocol === 'file:' &&
        actual.host === expectedUrl.host &&
        actual.pathname === expectedUrl.pathname
      );
    }
    return actual.origin === expectedUrl.origin && actual.pathname === expectedUrl.pathname;
  } catch {
    return false;
  }
}

export function isAuthorizedFileAccessRequest(
  provenance: FileAccessRequestProvenance,
  expectedUrl: URL
): boolean {
  return (
    provenance.isRegisteredWindow &&
    provenance.isMainFrame &&
    isAppRendererUrl(provenance.rendererUrl, expectedUrl)
  );
}

function isMissingFile(error: unknown): boolean {
  return (
    typeof error === 'object' &&
    error !== null &&
    'code' in error &&
    (error as { code?: unknown }).code === 'ENOENT'
  );
}

function missingFile(filePath: string): FileReadResult {
  return { file: '', filePath, error: null, found: false };
}

function failedRead(filePath: string, message: string): FileReadResult {
  return { file: '', filePath, error: message, found: false };
}

type WorkingDirectoryBinding =
  | { status: 'ready'; path: string }
  | { status: 'missing'; path: string }
  | { status: 'error'; path: string };

export class DesktopFileAccess {
  private readonly workingDirectories = new Map<number, WorkingDirectoryBinding>();

  async bindWindow(windowId: number, workingDirectory: string): Promise<void> {
    const resolvedPath = path.resolve(workingDirectory);
    try {
      const canonicalPath = await fs.realpath(resolvedPath);
      this.workingDirectories.set(windowId, { status: 'ready', path: canonicalPath });
    } catch (error) {
      this.workingDirectories.set(windowId, {
        status: isMissingFile(error) ? 'missing' : 'error',
        path: resolvedPath,
      });
    }
  }

  unbindWindow(windowId: number): void {
    this.workingDirectories.delete(windowId);
  }

  async readGoosehints(windowId: number): Promise<FileReadResult> {
    const binding = this.workingDirectories.get(windowId);
    if (!binding) {
      throw new Error('This window is not authorized to read .goosehints');
    }

    const filePath = path.join(binding.path, '.goosehints');
    if (binding.status === 'missing') {
      return missingFile(filePath);
    }
    if (binding.status === 'error') {
      return failedRead(filePath, 'Unable to resolve the working directory');
    }

    try {
      const metadata = await fs.lstat(filePath);
      if (metadata.isSymbolicLink()) {
        return failedRead(filePath, 'Refusing to read a symbolic link as .goosehints');
      }
      if (!metadata.isFile()) {
        return failedRead(filePath, '.goosehints is not a regular file');
      }

      const canonicalFilePath = await fs.realpath(filePath);
      if (path.dirname(canonicalFilePath) !== binding.path) {
        return failedRead(filePath, '.goosehints resolves outside the working directory');
      }

      const noFollow = process.platform === 'win32' ? 0 : fsConstants.O_NOFOLLOW;
      const handle = await fs.open(canonicalFilePath, fsConstants.O_RDONLY | noFollow);
      try {
        const openedMetadata = await handle.stat();
        if (!openedMetadata.isFile()) {
          return failedRead(filePath, '.goosehints is not a regular file');
        }
        if (openedMetadata.dev !== metadata.dev || openedMetadata.ino !== metadata.ino) {
          return failedRead(filePath, '.goosehints changed while it was being opened');
        }
        return {
          file: await handle.readFile('utf8'),
          filePath,
          error: null,
          found: true,
        };
      } finally {
        await handle.close();
      }
    } catch (error) {
      if (isMissingFile(error)) {
        return missingFile(filePath);
      }
      return failedRead(filePath, 'Unable to read .goosehints');
    }
  }
}

export async function readSelectedRecipe(filePath: string): Promise<FileReadResult> {
  const extension = path.extname(filePath).toLowerCase();
  if (extension !== '.yaml' && extension !== '.yml') {
    return failedRead(filePath, 'The selected recipe must be a YAML file');
  }

  try {
    const nonBlocking = process.platform === 'win32' ? 0 : fsConstants.O_NONBLOCK;
    const handle = await fs.open(filePath, fsConstants.O_RDONLY | nonBlocking);
    try {
      const metadata = await handle.stat();
      if (!metadata.isFile()) {
        return failedRead(filePath, 'The selected recipe is not a regular file');
      }
      return {
        file: await handle.readFile('utf8'),
        filePath,
        error: null,
        found: true,
      };
    } finally {
      await handle.close();
    }
  } catch (error) {
    if (isMissingFile(error)) {
      return missingFile(filePath);
    }
    return failedRead(filePath, 'Unable to read the selected recipe');
  }
}
