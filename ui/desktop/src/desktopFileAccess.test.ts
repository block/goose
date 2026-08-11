import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { afterEach, describe, expect, it } from 'vitest';
import {
  DesktopFileAccess,
  isAppRendererUrl,
  isAuthorizedFileAccessRequest,
  readSelectedRecipe,
} from './desktopFileAccess';

const tempDirectories: string[] = [];

function makeTempDirectory(): string {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'goose-desktop-file-access-'));
  tempDirectories.push(directory);
  return directory;
}

afterEach(() => {
  while (tempDirectories.length > 0) {
    fs.rmSync(tempDirectories.pop()!, { recursive: true, force: true });
  }
});

describe('DesktopFileAccess', () => {
  it('reads .goosehints from the bound working directory', async () => {
    const workingDirectory = makeTempDirectory();
    fs.writeFileSync(path.join(workingDirectory, '.goosehints'), 'project guidance');
    const access = new DesktopFileAccess();
    access.bindWindow(7, workingDirectory);
    const canonicalWorkingDirectory = fs.realpathSync(workingDirectory);

    await expect(access.readGoosehints(7)).resolves.toEqual({
      file: 'project guidance',
      filePath: path.join(canonicalWorkingDirectory, '.goosehints'),
      error: null,
      found: true,
    });
  });

  it('preserves missing-file behavior', async () => {
    const workingDirectory = makeTempDirectory();
    const access = new DesktopFileAccess();
    access.bindWindow(7, workingDirectory);
    const canonicalWorkingDirectory = fs.realpathSync(workingDirectory);

    await expect(access.readGoosehints(7)).resolves.toEqual({
      file: '',
      filePath: path.join(canonicalWorkingDirectory, '.goosehints'),
      error: null,
      found: false,
    });
  });

  it('rejects a renderer without a bound working directory', async () => {
    const access = new DesktopFileAccess();

    await expect(access.readGoosehints(99)).rejects.toThrow('not authorized');
  });

  it.skipIf(process.platform === 'win32')(
    'blocks a .goosehints symlink that escapes the working directory',
    async () => {
      const root = makeTempDirectory();
      const workingDirectory = path.join(root, 'project');
      const secretPath = path.join(root, 'secret');
      fs.mkdirSync(workingDirectory);
      fs.writeFileSync(secretPath, 'host secret');
      fs.symlinkSync('../secret', path.join(workingDirectory, '.goosehints'));
      const access = new DesktopFileAccess();
      access.bindWindow(7, workingDirectory);

      const result = await access.readGoosehints(7);

      expect(result.found).toBe(false);
      expect(result.file).toBe('');
      expect(result.error).toContain('symbolic link');
    }
  );
});

describe('renderer provenance', () => {
  const devServerUrl = new URL('http://127.0.0.1:5173/');

  it('accepts legitimate hash-routed app URLs', () => {
    expect(isAppRendererUrl('http://127.0.0.1:5173/#/settings', devServerUrl)).toBe(true);
    expect(isAppRendererUrl('http://127.0.0.1:5173/#/schedules?tab=active', devServerUrl)).toBe(
      true
    );
    expect(
      isAppRendererUrl(
        'file:///Applications/Goose.app/Contents/Resources/renderer/main_window/index.html#/settings',
        new URL('file:///Applications/Goose.app/Contents/Resources/renderer/main_window/index.html')
      )
    ).toBe(true);
  });

  it('rejects sibling paths, foreign origins, and malformed URLs', () => {
    expect(isAppRendererUrl('http://127.0.0.1:5173/admin#/settings', devServerUrl)).toBe(false);
    expect(isAppRendererUrl('http://localhost:5173/#/settings', devServerUrl)).toBe(false);
    expect(isAppRendererUrl('https://attacker.example/#/settings', devServerUrl)).toBe(false);
    expect(isAppRendererUrl('not a URL', devServerUrl)).toBe(false);
  });

  it('requires a registered top-level Goose window', () => {
    const legitimateRequest = {
      isRegisteredWindow: true,
      isMainFrame: true,
      rendererUrl: 'http://127.0.0.1:5173/#/settings',
    };

    expect(isAuthorizedFileAccessRequest(legitimateRequest, devServerUrl)).toBe(true);
    expect(
      isAuthorizedFileAccessRequest(
        { ...legitimateRequest, isRegisteredWindow: false },
        devServerUrl
      )
    ).toBe(false);
    expect(
      isAuthorizedFileAccessRequest({ ...legitimateRequest, isMainFrame: false }, devServerUrl)
    ).toBe(false);
  });
});

describe('readSelectedRecipe', () => {
  it('reads a picker-selected YAML recipe', async () => {
    const directory = makeTempDirectory();
    const recipePath = path.join(directory, 'recipe.yaml');
    fs.writeFileSync(recipePath, 'title: Daily summary');

    await expect(readSelectedRecipe(recipePath)).resolves.toEqual({
      file: 'title: Daily summary',
      filePath: recipePath,
      error: null,
      found: true,
    });
  });

  it('does not read a selected non-recipe file', async () => {
    const directory = makeTempDirectory();
    const secretPath = path.join(directory, 'secret.txt');
    fs.writeFileSync(secretPath, 'host secret');

    const result = await readSelectedRecipe(secretPath);

    expect(result.found).toBe(false);
    expect(result.file).toBe('');
    expect(result.error).toContain('YAML');
  });
});
