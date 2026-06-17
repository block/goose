import { describe, expect, it, vi } from 'vitest';
import { openDirectoryInExplorer } from './openDirectoryInExplorer';

describe('openDirectoryInExplorer', () => {
  it('returns an error when the target directory does not exist', async () => {
    const result = await openDirectoryInExplorer('/missing/skill', {
      directoryExists: () => false,
      execFile: vi.fn(),
      openPath: vi.fn(),
      platform: 'darwin',
    });

    expect(result).toEqual({
      error: 'Directory does not exist.',
      opened: false,
    });
  });

  it('uses the absolute macOS open command for directories on darwin', async () => {
    const execFile = vi.fn((_command, _args, callback: (error: Error | null) => void) =>
      callback(null)
    );

    const result = await openDirectoryInExplorer('/tmp/security-goose/.agents/skills/wooyun-legacy', {
      directoryExists: () => true,
      execFile,
      openPath: vi.fn(),
      platform: 'darwin',
    });

    expect(result).toEqual({ opened: true });
    expect(execFile).toHaveBeenCalledWith(
      '/usr/bin/open',
      ['-R', '/tmp/security-goose/.agents/skills/wooyun-legacy'],
      expect.any(Function)
    );
  });

  it('falls back to shell.openPath when the macOS open command fails', async () => {
    const openPath = vi.fn(async () => '');

    const result = await openDirectoryInExplorer('/tmp/security-goose/.agents/skills/wooyun-legacy', {
      directoryExists: () => true,
      execFile: vi.fn((_command, _args, callback: (error: Error | null) => void) =>
        callback(new Error('spawn open ENOENT'))
      ),
      openPath,
      platform: 'darwin',
    });

    expect(result).toEqual({ opened: true });
    expect(openPath).toHaveBeenCalledWith('/tmp/security-goose/.agents/skills/wooyun-legacy');
  });

  it('interprets shell.openPath empty-string success correctly on non-darwin platforms', async () => {
    const openPath = vi.fn(async () => '');

    const result = await openDirectoryInExplorer('/tmp/security-goose/.agents/skills/wooyun-legacy', {
      directoryExists: () => true,
      execFile: vi.fn(),
      openPath,
      platform: 'linux',
    });

    expect(result).toEqual({ opened: true });
    expect(openPath).toHaveBeenCalledWith('/tmp/security-goose/.agents/skills/wooyun-legacy');
  });

  it('returns the openPath error message when shell.openPath fails', async () => {
    const result = await openDirectoryInExplorer('/tmp/security-goose/.agents/skills/wooyun-legacy', {
      directoryExists: () => true,
      execFile: vi.fn(),
      openPath: vi.fn(async () => 'failed'),
      platform: 'linux',
    });

    expect(result).toEqual({
      error: 'failed',
      opened: false,
    });
  });

  it('returns the macOS open command error when the Finder handoff fails', async () => {
    const result = await openDirectoryInExplorer('/tmp/security-goose/.agents/skills/wooyun-legacy', {
      directoryExists: () => true,
      execFile: vi.fn((_command, _args, callback: (error: Error | null) => void) =>
        callback(new Error('Permission denied'))
      ),
      openPath: vi.fn(),
      platform: 'darwin',
    });

    expect(result).toEqual({
      error: 'Permission denied',
      opened: false,
    });
  });
});
