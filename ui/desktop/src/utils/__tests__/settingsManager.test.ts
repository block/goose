import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import fsSync from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { getSettings, updateSettings, defaultSettings } from '../settingsManager';

// Each test gets its own temp directory to avoid interference
let tmpDir: string;
let settingsFile: string;

beforeEach(() => {
  tmpDir = fsSync.mkdtempSync(path.join(os.tmpdir(), 'goose-settings-test-'));
  settingsFile = path.join(tmpDir, 'settings.json');
});

afterEach(() => {
  fsSync.rmSync(tmpDir, { recursive: true, force: true });
});

describe('getSettings — bug verification', () => {
  it('returns default settings when settings file does not exist', () => {
    const result = getSettings(settingsFile);
    expect(result).toEqual(defaultSettings);
  });

  it('reads valid settings from file', () => {
    const custom = { ...defaultSettings, showDockIcon: false };
    fsSync.writeFileSync(settingsFile, JSON.stringify(custom, null, 2));
    const result = getSettings(settingsFile);
    expect(result.showDockIcon).toBe(false);
  });

  // BUG: corrupted JSON should NOT crash the app — it should return defaults.
  // This test verifies the bug exists by checking that getSettings throws.
  it('should NOT throw on corrupted JSON (returns defaults instead)', () => {
    fsSync.writeFileSync(settingsFile, '{corrupted json!!!');
    // If getSettings throws here, the bug is confirmed — JSON.parse is unguarded
    expect(() => getSettings(settingsFile)).not.toThrow();
    expect(getSettings(settingsFile)).toEqual(defaultSettings);
  });

  it('should NOT throw on empty file (returns defaults instead)', () => {
    fsSync.writeFileSync(settingsFile, '');
    expect(() => getSettings(settingsFile)).not.toThrow();
    expect(getSettings(settingsFile)).toEqual(defaultSettings);
  });

  it('should NOT throw on truncated JSON (returns defaults instead)', () => {
    fsSync.writeFileSync(settingsFile, '{"showMenuBarIcon": tru');
    expect(() => getSettings(settingsFile)).not.toThrow();
    expect(getSettings(settingsFile)).toEqual(defaultSettings);
  });
});

describe('updateSettings — bug verification', () => {
  it('persists a settings change', () => {
    const result = getSettings(settingsFile);
    expect(result).toEqual(defaultSettings);

    updateSettings(settingsFile, (s) => {
      s.enableWakelock = true;
    });

    const updated = getSettings(settingsFile);
    expect(updated.enableWakelock).toBe(true);
  });

  // BUG: updateSettings writes directly to the target path with writeFileSync.
  // It should instead write to a temp file and rename for atomicity.
  // We verify the bug by spying on fs and checking the call pattern.
  it('should use atomic write (temp file + rename) instead of direct writeFileSync', () => {
    const writeSpy = vi.spyOn(fsSync, 'writeFileSync');
    const renameSpy = vi.spyOn(fsSync, 'renameSync');

    updateSettings(settingsFile, (s) => {
      s.showMenuBarIcon = false;
    });

    // If the write is atomic, writeFileSync should target a .tmp file,
    // and renameSync should move it to the final path.
    const writeCall = writeSpy.mock.calls[0];
    const writtenPath = writeCall[0] as string;

    // Check that renameSync was called (atomic pattern)
    expect(renameSpy).toHaveBeenCalled();

    // Check that writeFileSync wrote to a temp file, NOT the settings file directly
    expect(writtenPath).not.toBe(settingsFile);
    expect(writtenPath).toContain('.tmp');

    writeSpy.mockRestore();
    renameSpy.mockRestore();
  });
});
