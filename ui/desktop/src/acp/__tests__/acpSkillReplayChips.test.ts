import { describe, expect, it, beforeEach } from 'vitest';
import {
  attachChipsToMessage,
  clearSkillReplayChips,
  getPendingReplayChips,
  isAssistantOnlyAudience,
  setPendingReplayChips,
  skillInstructionToChips,
} from '../acpSkillReplayChips';
import type { Message } from '../../types/message';

describe('acpSkillReplayChips', () => {
  beforeEach(() => {
    clearSkillReplayChips();
  });

  it('parses CLI-aligned skill instructions into chips', () => {
    expect(
      skillInstructionToChips(
        'Use the load_skill tool to load the following skills: "code-review", "insight".'
      )
    ).toEqual([
      { label: 'code-review', type: 'skill' },
      { label: 'insight', type: 'skill' },
    ]);
  });

  it('detects assistant-only audience annotations', () => {
    expect(isAssistantOnlyAudience({ audience: ['assistant'] })).toBe(true);
    expect(isAssistantOnlyAudience({ audience: ['user', 'assistant'] })).toBe(false);
  });

  it('stores pending chips and attaches them to messages without duplicates', () => {
    setPendingReplayChips('msg-1', [{ label: 'review', type: 'skill' }]);
    expect(getPendingReplayChips('msg-1')).toEqual([{ label: 'review', type: 'skill' }]);

    const message: Message = {
      id: 'msg-1',
      role: 'user',
      created: 1,
      content: [{ type: 'text', text: 'hello' }],
      metadata: {
        userVisible: true,
        agentVisible: true,
        chips: [{ label: 'review', type: 'skill' }],
      },
    };
    attachChipsToMessage(message, [
      { label: 'review', type: 'skill' },
      { label: 'insight', type: 'skill' },
    ]);
    expect(message.metadata.chips).toEqual([
      { label: 'review', type: 'skill' },
      { label: 'insight', type: 'skill' },
    ]);
  });
});
