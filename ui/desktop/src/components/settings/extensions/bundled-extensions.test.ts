import { describe, it, expect, vi } from 'vitest';
import { isDeprecatedGoogleDriveExtension, syncBundledExtensions } from './bundled-extensions';
import type { FixedExtensionEntry } from '../../ConfigContext';

vi.mock('./bundled-extensions.json', () => ({
  default: [
    {
      id: 'developer',
      name: 'developer',
      display_name: 'Developer',
      description: 'General development tools.',
      enabled: true,
      type: 'builtin',
      timeout: 300,
    },
    {
      id: 'googledrive',
      name: 'googledrive',
      display_name: 'Google Drive',
      description: 'Google Drive integration.',
      enabled: true,
      type: 'stdio',
      cmd: 'googledrive-mcp',
      args: [],
      env_keys: [],
      timeout: 300,
    },
  ],
}));

describe('isDeprecatedGoogleDriveExtension', () => {
  it('returns true for builtin googledrive', () => {
    const ext = {
      name: 'Google Drive',
      type: 'builtin',
      description: 'Google Drive extension',
      enabled: true,
      bundled: true,
    } as FixedExtensionEntry;
    expect(isDeprecatedGoogleDriveExtension(ext)).toBe(true);
  });

  it('returns true for builtin google_drive', () => {
    const ext = {
      name: 'google_drive',
      type: 'builtin',
      description: 'Google Drive extension',
      enabled: true,
      bundled: true,
    } as FixedExtensionEntry;
    expect(isDeprecatedGoogleDriveExtension(ext)).toBe(true);
  });

  it('returns true for stdio googledrive with GOOGLE_DRIVE_CREDENTIALS_PATH', () => {
    const ext = {
      name: 'Google Drive',
      type: 'stdio',
      description: 'Google Drive extension',
      cmd: 'some-cmd',
      args: [],
      env_keys: ['GOOGLE_DRIVE_CREDENTIALS_PATH'],
      enabled: true,
      bundled: true,
    } as FixedExtensionEntry;
    expect(isDeprecatedGoogleDriveExtension(ext)).toBe(true);
  });

  it('returns true for stdio googledrive with GOOGLE_DRIVE_OAUTH_PATH', () => {
    const ext = {
      name: 'Google Drive',
      type: 'stdio',
      description: 'Google Drive extension',
      cmd: 'some-cmd',
      args: [],
      env_keys: ['GOOGLE_DRIVE_OAUTH_PATH'],
      enabled: true,
      bundled: true,
    } as FixedExtensionEntry;
    expect(isDeprecatedGoogleDriveExtension(ext)).toBe(true);
  });

  it('returns false for stdio googledrive without deprecated env keys', () => {
    const ext = {
      name: 'Google Drive',
      type: 'stdio',
      description: 'Google Drive extension',
      cmd: 'some-cmd',
      args: [],
      env_keys: [],
      enabled: true,
      bundled: true,
    } as FixedExtensionEntry;
    expect(isDeprecatedGoogleDriveExtension(ext)).toBe(false);
  });

  it('returns false for non-googledrive extensions', () => {
    const ext = {
      name: 'developer',
      type: 'builtin',
      description: 'Developer tools',
      enabled: true,
      bundled: true,
    } as FixedExtensionEntry;
    expect(isDeprecatedGoogleDriveExtension(ext)).toBe(false);
  });

  it('returns false for non-googledrive stdio with those env keys', () => {
    const ext = {
      name: 'some-other-ext',
      type: 'stdio',
      description: 'Other extension',
      cmd: 'some-cmd',
      args: [],
      env_keys: ['GOOGLE_DRIVE_CREDENTIALS_PATH'],
      enabled: true,
      bundled: true,
    } as FixedExtensionEntry;
    expect(isDeprecatedGoogleDriveExtension(ext)).toBe(false);
  });
});

describe('syncBundledExtensions', () => {
  it('overwrites deprecated builtin googledrive extension', async () => {
    const addExtensionFn = vi.fn().mockResolvedValue(undefined);
    const existingExtensions = [
      {
        name: 'googledrive',
        type: 'builtin',
        description: 'Google Drive',
        enabled: true,
        bundled: true,
      },
    ] as FixedExtensionEntry[];

    await syncBundledExtensions(existingExtensions, addExtensionFn);

    expect(addExtensionFn).toHaveBeenCalledWith(
      'googledrive',
      expect.objectContaining({ type: 'stdio', bundled: true }),
      true
    );
  });

  it('overwrites stdio googledrive with deprecated env keys', async () => {
    const addExtensionFn = vi.fn().mockResolvedValue(undefined);
    const existingExtensions = [
      {
        name: 'googledrive',
        type: 'stdio',
        description: 'Google Drive',
        cmd: 'some-cmd',
        args: [],
        env_keys: ['GOOGLE_DRIVE_CREDENTIALS_PATH'],
        enabled: true,
        bundled: true,
      },
    ] as FixedExtensionEntry[];

    await syncBundledExtensions(existingExtensions, addExtensionFn);

    expect(addExtensionFn).toHaveBeenCalledWith(
      'googledrive',
      expect.objectContaining({ type: 'stdio', bundled: true, env_keys: [] }),
      true
    );
  });

  it('skips already bundled non-deprecated extensions', async () => {
    const addExtensionFn = vi.fn().mockResolvedValue(undefined);
    const existingExtensions = [
      {
        name: 'developer',
        type: 'builtin',
        description: 'Developer tools',
        enabled: true,
        bundled: true,
        timeout: 300,
      },
    ] as FixedExtensionEntry[];

    await syncBundledExtensions(existingExtensions, addExtensionFn);

    expect(addExtensionFn).not.toHaveBeenCalledWith(
      'developer',
      expect.anything(),
      expect.anything()
    );
  });
});
