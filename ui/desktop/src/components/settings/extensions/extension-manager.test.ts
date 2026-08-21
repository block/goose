import { describe, expect, it, vi } from 'vitest';
import type { ExtensionConfig } from '../../../types/extensions';
import { changesExtensionIdentity, renameExtensionDefault } from './extension-manager';

const renamedExtension: ExtensionConfig = {
  type: 'builtin',
  name: 'Renamed',
  description: '',
};

describe('renameExtensionDefault', () => {
  it('preserves the original extension when adding the replacement fails', async () => {
    const addError = new Error('identity already exists');
    const addToConfig = vi.fn().mockRejectedValue(addError);
    const removeFromConfig = vi.fn();

    await expect(
      renameExtensionDefault({
        originalName: 'Original',
        extensionConfig: renamedExtension,
        enabled: true,
        addToConfig,
        removeFromConfig,
      })
    ).rejects.toBe(addError);

    expect(removeFromConfig).not.toHaveBeenCalled();
  });

  it('removes the original only after the replacement is persisted', async () => {
    const operations: string[] = [];
    const addToConfig = vi.fn().mockImplementation(async () => {
      operations.push('add');
    });
    const removeFromConfig = vi.fn().mockImplementation(async () => {
      operations.push('remove');
    });

    await renameExtensionDefault({
      originalName: 'Original',
      extensionConfig: renamedExtension,
      enabled: false,
      addToConfig,
      removeFromConfig,
    });

    expect(operations).toEqual(['add', 'remove']);
    expect(addToConfig).toHaveBeenCalledWith('Renamed', renamedExtension, false);
    expect(removeFromConfig).toHaveBeenCalledWith('Original');
  });
});

describe('changesExtensionIdentity', () => {
  it('treats capitalization and whitespace aliases as the same identity', () => {
    expect(changesExtensionIdentity('github', 'Git Hub')).toBe(false);
    expect(changesExtensionIdentity('github', 'gitlab')).toBe(true);
  });
});
