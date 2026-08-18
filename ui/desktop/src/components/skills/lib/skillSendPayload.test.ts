import { describe, expect, it } from 'vitest';
import { buildSkillSendPayload } from './skillSendPayload';

describe('buildSkillSendPayload', () => {
  it('returns plain text when no skills are selected', () => {
    expect(buildSkillSendPayload('hello', [], null)).toEqual({ messageText: 'hello' });
  });

  it('builds assistant-only instruction and chips for selected skills', () => {
    const result = buildSkillSendPayload(
      'Review this PR',
      [
        { id: 'code-review', name: 'code-review' },
        { id: 'insight', name: 'insight' },
      ],
      null
    );

    expect(result.messageText).toBe('Review this PR');
    expect(result.sendOptions?.chips).toEqual([
      { label: 'code-review', type: 'skill' },
      { label: 'insight', type: 'skill' },
    ]);
    expect(result.sendOptions?.assistantPrompt).toBe(
      'Use the load_skill tool to load the following skills: "code-review", "insight".'
    );
  });

  it('uses slash skill match when no drafts are present', () => {
    const result = buildSkillSendPayload('/review fix tests', [], {
      skill: { name: 'review' },
      promptText: 'Use the review skill to fix tests',
      displayText: 'fix tests',
    });

    expect(result.messageText).toBe('fix tests');
    expect(result.sendOptions?.chips).toEqual([{ label: 'review', type: 'skill' }]);
  });
});
