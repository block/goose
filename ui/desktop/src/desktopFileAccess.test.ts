import fs, { constants as fsConstants } from 'node:fs';
import fsPromises from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { afterEach, describe, expect, it, vi } from 'vitest';
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
  vi.restoreAllMocks();
  while (tempDirectories.length > 0) {
    fs.rmSync(tempDirectories.pop()!, { recursive: true, force: true });
  }
});

describe('DesktopFileAccess', () => {
  it('reads .goosehints from the bound working directory', async () => {
    const workingDirectory = makeTempDirectory();
    fs.writeFileSync(path.join(workingDirectory, '.goosehints'), 'project guidance');
    const access = new DesktopFileAccess();
    await access.bindWindow(7, workingDirectory);
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
    await access.bindWindow(7, workingDirectory);
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
      await access.bindWindow(7, workingDirectory);

      const result = await access.readGoosehints(7);

      expect(result.found).toBe(false);
      expect(result.file).toBe('');
      expect(result.error).toContain('symbolic link');
    }
  );

  it.skipIf(process.platform === 'win32')(
    'keeps a symlinked working directory pinned to its bind-time target',
    async () => {
      const root = makeTempDirectory();
      const firstProject = path.join(root, 'first-project');
      const secondProject = path.join(root, 'second-project');
      const workingDirectory = path.join(root, 'current-project');
      fs.mkdirSync(firstProject);
      fs.mkdirSync(secondProject);
      fs.writeFileSync(path.join(firstProject, '.goosehints'), 'first guidance');
      fs.writeFileSync(path.join(secondProject, '.goosehints'), 'second guidance');
      fs.symlinkSync(firstProject, workingDirectory);
      const access = new DesktopFileAccess();
      await access.bindWindow(7, workingDirectory);
      const canonicalFirstProject = fs.realpathSync(firstProject);

      fs.unlinkSync(workingDirectory);
      fs.symlinkSync(secondProject, workingDirectory);

      await expect(access.readGoosehints(7)).resolves.toEqual({
        file: 'first guidance',
        filePath: path.join(canonicalFirstProject, '.goosehints'),
        error: null,
        found: true,
      });
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
    expect(
      isAppRendererUrl(
        'file://attacker/Applications/Goose.app/Contents/Resources/renderer/main_window/index.html',
        new URL('file:///Applications/Goose.app/Contents/Resources/renderer/main_window/index.html')
      )
    ).toBe(false);
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

  it.skipIf(process.platform === 'win32')('allows a picker-selected YAML symlink', async () => {
    const directory = makeTempDirectory();
    const targetPath = path.join(directory, 'target.yaml');
    const recipePath = path.join(directory, 'recipe.yaml');
    fs.writeFileSync(targetPath, 'title: Linked recipe');
    fs.symlinkSync(targetPath, recipePath);

    await expect(readSelectedRecipe(recipePath)).resolves.toEqual({
      file: 'title: Linked recipe',
      filePath: recipePath,
      error: null,
      found: true,
    });
  });

  it.skipIf(process.platform === 'win32')(
    'reads from the opened recipe when a selected symlink is retargeted',
    async () => {
      const directory = makeTempDirectory();
      const firstTarget = path.join(directory, 'first.yaml');
      const secondTarget = path.join(directory, 'second.yaml');
      const recipePath = path.join(directory, 'recipe.yaml');
      fs.writeFileSync(firstTarget, 'title: First recipe');
      fs.writeFileSync(secondTarget, 'title: Second recipe');
      fs.symlinkSync(firstTarget, recipePath);
      const open = fsPromises.open.bind(fsPromises);
      const openSpy = vi.spyOn(fsPromises, 'open').mockImplementationOnce(async (...args) => {
        const handle = await open(...args);
        fs.unlinkSync(recipePath);
        fs.symlinkSync(secondTarget, recipePath);
        return handle;
      });

      await expect(readSelectedRecipe(recipePath)).resolves.toEqual({
        file: 'title: First recipe',
        filePath: recipePath,
        error: null,
        found: true,
      });
      expect(openSpy).toHaveBeenCalledOnce();
    }
  );

  it.skipIf(process.platform === 'win32')(
    'rejects a picker-selected FIFO without blocking',
    async () => {
      const directory = makeTempDirectory();
      const recipePath = path.join(directory, 'recipe.yaml');
      execFileSync('mkfifo', [recipePath]);
      const openSpy = vi.spyOn(fsPromises, 'open');

      const result = await readSelectedRecipe(recipePath);

      expect(result.found).toBe(false);
      expect(result.error).toContain('not a regular file');
      expect(openSpy).toHaveBeenCalledWith(
        recipePath,
        fsConstants.O_RDONLY | fsConstants.O_NONBLOCK
      );
    }
  );
});
