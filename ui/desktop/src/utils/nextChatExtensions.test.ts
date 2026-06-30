import { describe, expect, it } from 'vitest';
import {
  createNextChatExtensionDraft,
  selectNextChatExtensions,
  toggleNextChatExtension,
} from './nextChatExtensions';
import type { FixedExtensionEntry } from '../components/ConfigContext';

const extension = (name: string, enabled: boolean): FixedExtensionEntry => ({
  name,
  enabled,
  type: 'builtin',
  description: `${name} extension`,
});

describe('nextChatExtensions', () => {
  it('creates a draft from enabled configured extensions', () => {
    const draft = createNextChatExtensionDraft([
      extension('developer', true),
      extension('memory', false),
    ]);

    expect([...draft.selectedNames]).toEqual(['developer']);
  });

  it('toggles selected extension names', () => {
    const draft = createNextChatExtensionDraft([extension('developer', true)]);

    const withoutDeveloper = toggleNextChatExtension(draft, extension('developer', true));
    expect(withoutDeveloper.selectedNames.has('developer')).toBe(false);

    const withMemory = toggleNextChatExtension(withoutDeveloper, extension('memory', false));
    expect([...withMemory.selectedNames]).toEqual(['memory']);
  });

  it('selects extension configs without the enabled field', () => {
    const extensions = [extension('developer', true), extension('memory', false)];
    const selected = selectNextChatExtensions(extensions, {
      selectedNames: new Set(['memory']),
    });

    expect(selected).toEqual([
      {
        name: 'memory',
        type: 'builtin',
        description: 'memory extension',
      },
    ]);
  });
});
