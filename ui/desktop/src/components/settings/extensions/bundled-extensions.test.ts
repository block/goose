import { describe, it, expect, vi } from 'vitest';
import { pruneDeprecatedBundledExtensions, syncBundledExtensions } from './bundled-extensions';
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

vi.mock('./deprecated-bundled-extensions.json', () => ({
  default: [{ id: 'googledrive' }, { id: 'old-bundled-extension' }],
}));

describe('syncBundledExtensions', () => {
  it('skips already bundled non-deprecated extensions', async () => {
    const addExtensionFn = vi.fn().mockResolvedValue(undefined);
    const updateExtensionFn = vi.fn().mockResolvedValue(undefined);
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

    await syncBundledExtensions(existingExtensions, addExtensionFn, updateExtensionFn);

    expect(addExtensionFn).not.toHaveBeenCalledWith(
      'developer',
      expect.anything(),
      expect.anything()
    );
    expect(updateExtensionFn).not.toHaveBeenCalled();
  });

  it('updates a matching legacy extension through the explicit update path', async () => {
    const addExtensionFn = vi.fn().mockResolvedValue(undefined);
    const updateExtensionFn = vi.fn().mockResolvedValue(undefined);
    const existingExtensions = [
      {
        name: 'Developer',
        configKey: 'developer',
        type: 'builtin',
        description: 'Legacy developer tools',
        enabled: false,
        bundled: false,
        timeout: 30,
      },
    ] as FixedExtensionEntry[];

    await syncBundledExtensions(existingExtensions, addExtensionFn, updateExtensionFn);

    expect(addExtensionFn).not.toHaveBeenCalledWith(
      'developer',
      expect.anything(),
      expect.anything()
    );
    expect(updateExtensionFn).toHaveBeenCalledWith(
      'developer',
      expect.objectContaining({
        type: 'builtin',
        name: 'developer',
        bundled: true,
      }),
      false
    );
  });
});

describe('pruneDeprecatedBundledExtensions', () => {
  it('removes deprecated bundled extensions', async () => {
    const removeExtensionFn = vi.fn().mockResolvedValue(undefined);
    const existingExtensions = [
      {
        name: 'old-bundled-extension',
        type: 'builtin',
        description: 'Old bundled extension',
        enabled: true,
        bundled: true,
      },
    ] as FixedExtensionEntry[];

    const remainingExtensions = await pruneDeprecatedBundledExtensions(
      existingExtensions,
      removeExtensionFn
    );

    expect(removeExtensionFn).toHaveBeenCalledWith('old-bundled-extension');
    expect(remainingExtensions).toEqual([]);
  });

  it('does not remove non-bundled deprecated extensions', async () => {
    const removeExtensionFn = vi.fn().mockResolvedValue(undefined);
    const existingExtensions = [
      {
        name: 'old-bundled-extension',
        type: 'builtin',
        description: 'Old bundled extension',
        enabled: true,
        bundled: false,
      },
    ] as FixedExtensionEntry[];

    const remainingExtensions = await pruneDeprecatedBundledExtensions(
      existingExtensions,
      removeExtensionFn
    );

    expect(removeExtensionFn).not.toHaveBeenCalled();
    expect(remainingExtensions).toEqual(existingExtensions);
  });

  it('allows same-id bundled extensions to be re-added after prune', async () => {
    const removeExtensionFn = vi.fn().mockResolvedValue(undefined);
    const addExtensionFn = vi.fn().mockResolvedValue(undefined);
    const updateExtensionFn = vi.fn().mockResolvedValue(undefined);
    const existingExtensions = [
      {
        name: 'Google Drive',
        type: 'stdio',
        description: 'Google Drive extension',
        cmd: 'some-cmd',
        args: [],
        env_keys: [],
        enabled: true,
        bundled: true,
      },
    ] as FixedExtensionEntry[];

    const remainingExtensions = await pruneDeprecatedBundledExtensions(
      existingExtensions,
      removeExtensionFn
    );

    await syncBundledExtensions(remainingExtensions, addExtensionFn, updateExtensionFn);

    expect(removeExtensionFn).toHaveBeenCalledWith('googledrive');
    expect(addExtensionFn).toHaveBeenCalledWith(
      'googledrive',
      expect.objectContaining({
        type: 'stdio',
        name: 'googledrive',
        bundled: true,
      }),
      true
    );
    expect(updateExtensionFn).not.toHaveBeenCalled();
  });
});
