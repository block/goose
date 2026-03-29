import { describe, expect, it } from 'vitest';
import { getDisplayItemInsertText, type DisplayItem } from './MentionPopover';

describe('getDisplayItemInsertText', () => {
  it('formats skill items as slash commands', () => {
    const item: DisplayItem = {
      name: 'some-skill',
      extra: 'A custom skill',
      itemType: 'Skill',
      relativePath: 'some-skill',
    };

    expect(getDisplayItemInsertText(item)).toBe('/some-skill');
  });

  it('formats builtin and recipe items as slash commands', () => {
    expect(
      getDisplayItemInsertText({
        name: 'clear',
        extra: 'Clear the conversation history',
        itemType: 'Builtin',
        relativePath: 'clear',
      })
    ).toBe('/clear');

    expect(
      getDisplayItemInsertText({
        name: 'my-recipe',
        extra: 'My recipe',
        itemType: 'Recipe',
        relativePath: 'my-recipe',
      })
    ).toBe('/my-recipe');
  });

  it('keeps file items as raw paths', () => {
    const item: DisplayItem = {
      name: 'Cargo.toml',
      extra: '/tmp/project/Cargo.toml',
      itemType: 'File',
      relativePath: 'Cargo.toml',
    };

    expect(getDisplayItemInsertText(item)).toBe('/tmp/project/Cargo.toml');
  });
});
