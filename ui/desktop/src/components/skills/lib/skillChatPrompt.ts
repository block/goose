export type SkillLike = { name: string };
export type SkillDraftLike = { name: string };

export type ChatSkillDraft = {
  id: string;
  name: string;
  description?: string;
  sourceLabel?: string;
};

export type SkillCommandMatch<TSkill extends SkillLike = SkillLike> = {
  skill: TSkill;
  promptText: string;
  displayText: string;
};

/** CLI-aligned progressive load nudge (matches format_load_skills_nudge). */
export const SKILL_INSTRUCTION_PREFIX =
  'Use the load_skill tool to load the following skills:';

const LEGACY_SKILL_INSTRUCTION_PREFIX = 'Use these skills for this request:';

const RESERVED_SLASH_COMMANDS = new Set([
  'clear',
  'compact',
  'doctor',
  'prompt',
  'prompts',
  'skills',
  'goal',
  'grind',
  'status',
]);

export function isReservedSlashCommand(command: string): boolean {
  return RESERVED_SLASH_COMMANDS.has(command.trim().toLowerCase());
}

export function formatSkillChatPrompt(skillName: string, taskText = ''): string {
  const name = skillName.trim();
  const task = taskText.trimStart();
  if (!task) {
    return `Use the ${name} skill`;
  }
  return `Use the ${name} skill to ${task}`;
}

export function formatSkillDraftsChatPrompt(skills: SkillDraftLike[], taskText = ''): string {
  if (skills.length === 0) {
    return taskText;
  }

  if (skills.length === 1) {
    return formatSkillChatPrompt(skills[0].name, taskText);
  }

  const skillNames = skills
    .map((skill) => skill.name.trim())
    .filter(Boolean)
    .join(', ');
  const task = taskText.trimStart();
  if (!task) {
    return `Use the ${skillNames} skills`;
  }
  return `Use the ${skillNames} skills to ${task}`;
}

export function formatSkillInstructionPrompt(skills: SkillDraftLike[]): string {
  const skillNames = skills
    .map((skill) => skill.name.trim())
    .filter(Boolean)
    .map((name) => `"${name}"`)
    .join(', ');
  return `${SKILL_INSTRUCTION_PREFIX} ${skillNames}.`;
}

export function parseSkillInstructionPrompt(text: string): string[] {
  const trimmed = text.trim();

  if (trimmed.startsWith(SKILL_INSTRUCTION_PREFIX)) {
    return trimmed
      .slice(SKILL_INSTRUCTION_PREFIX.length)
      .trim()
      .replace(/[.。]+$/, '')
      .split(',')
      .map((name) => name.trim().replace(/^["']|["']$/g, ''))
      .filter(Boolean);
  }

  if (trimmed.startsWith(LEGACY_SKILL_INSTRUCTION_PREFIX)) {
    return trimmed
      .slice(LEGACY_SKILL_INSTRUCTION_PREFIX.length)
      .trim()
      .replace(/[.。]+$/, '')
      .split(',')
      .map((name) => name.trim())
      .filter(Boolean);
  }

  return [];
}

export function toChatSkillDraft(skill: {
  id?: string;
  name: string;
  description?: string;
  sourceLabel?: string;
}): ChatSkillDraft {
  return {
    id: skill.id ?? skill.name,
    name: skill.name,
    description: skill.description,
    sourceLabel: skill.sourceLabel,
  };
}

export function expandSkillSlashCommand(text: string, skills: SkillLike[]): string | null {
  return resolveSkillSlashCommand(text, skills)?.promptText ?? null;
}

export function resolveSkillSlashCommand<TSkill extends SkillLike>(
  text: string,
  skills: TSkill[]
): SkillCommandMatch<TSkill> | null {
  const match = text.trimStart().match(/^\/(\S+)(?:\s+([\s\S]*))?$/);
  if (!match) {
    return null;
  }

  const command = match[1];
  if (isReservedSlashCommand(command)) {
    return null;
  }

  const skill = skills.find((candidate) => candidate.name.toLowerCase() === command.toLowerCase());
  if (!skill) {
    return null;
  }

  const displayText = match[2]?.trimStart() ?? '';
  return {
    skill,
    promptText: formatSkillChatPrompt(skill.name, displayText),
    displayText,
  };
}
