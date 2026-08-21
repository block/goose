import { describe, expect, it, vi } from 'vitest';
import type { ExtensionConfig } from '../../../types/extensions';
import { changesExtensionIdentity, renameExtensionDefault } from './extension-manager';

const renamedExtension: ExtensionConfig = {
  type: 'builtin',
  name: 'Renamed',
  description: '',
};

describe('renameExtensionDefault', () => {
  it('propagates an atomic rename failure', async () => {
    const renameError = new Error('identity already exists');
    const renameInConfig = vi.fn().mockRejectedValue(renameError);

    await expect(
      renameExtensionDefault({
        originalConfigKey: 'original',
        extensionConfig: renamedExtension,
        enabled: true,
        renameInConfig,
      })
    ).rejects.toBe(renameError);

    expect(renameInConfig).toHaveBeenCalledOnce();
  });

  it('uses one backend operation for the original and replacement', async () => {
    const renameInConfig = vi.fn().mockResolvedValue(undefined);

    await renameExtensionDefault({
      originalConfigKey: 'original-alias',
      extensionConfig: renamedExtension,
      enabled: false,
      renameInConfig,
    });

    expect(renameInConfig).toHaveBeenCalledWith('original-alias', renamedExtension, false);
  });
});

describe('changesExtensionIdentity', () => {
  it('treats capitalization and whitespace aliases as the same identity', () => {
    expect(changesExtensionIdentity('github', 'Git Hub')).toBe(false);
    expect(changesExtensionIdentity('foo.bar', 'foo/bar')).toBe(false);
    expect(changesExtensionIdentity('github', 'gitlab')).toBe(true);
  });
});
