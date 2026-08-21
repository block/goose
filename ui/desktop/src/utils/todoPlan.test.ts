import { describe, expect, it } from 'vitest';
import { isTodoWriteToolName, parseTodoMarkdown, todoDepth } from './todoPlan';

describe('todoPlan', () => {
  it('recognizes prefixed and unprefixed todo write tools', () => {
    expect(isTodoWriteToolName('todo__todo_write')).toBe(true);
    expect(isTodoWriteToolName('todo_write')).toBe(true);
    expect(isTodoWriteToolName('calendar__todo_write')).toBe(false);
    expect(isTodoWriteToolName('other__todo_read')).toBe(false);
  });

  it('parses checklist status, nesting, and infers the current item', () => {
    const entries = parseTodoMarkdown(
      ['- [x] Finished', '- [ ] Current', '  - [ ] Nested', '- [ ] Later'].join('\n')
    );

    expect(entries.map(({ content, status }) => ({ content, status }))).toEqual([
      { content: 'Finished', status: 'completed' },
      { content: 'Current', status: 'in_progress' },
      { content: 'Nested', status: 'pending' },
      { content: 'Later', status: 'pending' },
    ]);
    expect(todoDepth(entries[2])).toBe(1);
  });

  it('preserves an explicitly marked current item', () => {
    const entries = parseTodoMarkdown('- [ ] Before\n- [>] Active\n- [ ] After');

    expect(entries.map((entry) => entry.status)).toEqual(['pending', 'in_progress', 'pending']);
  });
});
