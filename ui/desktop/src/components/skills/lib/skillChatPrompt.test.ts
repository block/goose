import { describe, expect, it } from 'vitest';
import {
  formatSkillInstructionPrompt,
  isReservedSlashCommand,
  parseSkillInstructionPrompt,
  resolveSkillSlashCommand,
} from './skillChatPrompt';

describe('skillChatPrompt', () => {
  it('formats CLI-aligned load_skill instruction for multiple skills', () => {
    expect(formatSkillInstructionPrompt([{ name: 'code-review' }, { name: 'insight' }])).toBe(
      'Use the load_skill tool to load the following skills: "code-review", "insight".'
    );
  });

  it('parses CLI and legacy instruction prompts', () => {
    expect(
      parseSkillInstructionPrompt(
        'Use the load_skill tool to load the following skills: "code-review", "insight".'
      )
    ).toEqual(['code-review', 'insight']);
    expect(parseSkillInstructionPrompt('Use these skills for this request: review, insight.')).toEqual(
      ['review', 'insight']
    );
  });

  it('resolves slash skill commands and skips reserved commands', () => {
    const skills = [{ name: 'review' }, { name: 'github:triage' }];
    expect(resolveSkillSlashCommand('/review fix tests', skills)).toMatchObject({
      skill: { name: 'review' },
      displayText: 'fix tests',
    });
    expect(resolveSkillSlashCommand('/github:triage this PR', skills)?.skill.name).toBe(
      'github:triage'
    );
    expect(isReservedSlashCommand('skills')).toBe(true);
    expect(resolveSkillSlashCommand('/skills review insight', skills)).toBeNull();
  });
});
