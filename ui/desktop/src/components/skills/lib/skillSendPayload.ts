import {
  formatSkillInstructionPrompt,
  type ChatSkillDraft,
  type SkillCommandMatch,
} from './skillChatPrompt';
import type { ChatSendOptions, MessageChip } from '../../../types/message';

export interface SkillSendPayload {
  messageText: string;
  sendOptions?: ChatSendOptions;
}

export function buildSkillSendPayload(
  submittedText: string,
  submittedSkills: ChatSkillDraft[],
  slashSkillCommand: SkillCommandMatch | null
): SkillSendPayload {
  const chips: MessageChip[] =
    submittedSkills.length > 0
      ? submittedSkills.map((skill) => ({
          label: skill.name,
          type: 'skill' as const,
        }))
      : slashSkillCommand
        ? [{ label: slashSkillCommand.skill.name, type: 'skill' as const }]
        : [];

  if (chips.length === 0) {
    return { messageText: submittedText };
  }

  const skillsForPrompt =
    submittedSkills.length > 0
      ? submittedSkills
      : slashSkillCommand
        ? [slashSkillCommand.skill]
        : [];
  const assistantPrompt = formatSkillInstructionPrompt(skillsForPrompt);
  const displayText =
    submittedSkills.length > 0 ? submittedText.trim() : (slashSkillCommand?.displayText ?? '');

  return {
    messageText: displayText || ' ',
    sendOptions: {
      chips,
      displayText,
      assistantPrompt,
    },
  };
}

export function buildSkillRetryOptions(
  text: string,
  chips?: MessageChip[]
): ChatSendOptions | undefined {
  const skillChips = chips?.filter((chip) => chip.type === 'skill') ?? [];
  if (skillChips.length === 0) return undefined;

  return {
    displayText: text,
    assistantPrompt: formatSkillInstructionPrompt(skillChips.map((chip) => ({ name: chip.label }))),
    chips: skillChips,
  };
}
