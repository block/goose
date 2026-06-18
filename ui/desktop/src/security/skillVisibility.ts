import type { SlashCommand } from '../api';

export const SECURITY_GOOSE_VISIBLE_SKILLS_SCOPE = 'builtin-and-security';

export const SECURITY_GOOSE_VISIBLE_SKILL_NAMES = [
  'goose-doc-guide',
  'alert-triage',
  'asset-risk-summary',
  'ioc-analysis',
  'report-writing',
  'vuln-triage',
  'wooyun-legacy',
] as const;

const SECURITY_GOOSE_VISIBLE_SKILL_NAME_SET = new Set<string>(SECURITY_GOOSE_VISIBLE_SKILL_NAMES);

export function getVisibleSkillsScope(): string | undefined {
  if (typeof window === 'undefined' || !window.appConfig) {
    return undefined;
  }

  const value = window.appConfig.get('GOOSE_VISIBLE_SKILLS_SCOPE');
  return typeof value === 'string' && value.length > 0 ? value : undefined;
}

export function isSecurityGooseScopedSkillVisibility(
  scope: string | undefined = getVisibleSkillsScope()
): boolean {
  return scope === SECURITY_GOOSE_VISIBLE_SKILLS_SCOPE;
}

export function filterVisibleSkillCommands(
  commands: SlashCommand[],
  scope: string | undefined = getVisibleSkillsScope(),
  extraVisibleSkillNames: Iterable<string> = []
): SlashCommand[] {
  if (!isSecurityGooseScopedSkillVisibility(scope)) {
    return commands;
  }

  const allowedSkillNames = new Set<string>([
    ...SECURITY_GOOSE_VISIBLE_SKILL_NAME_SET,
    ...extraVisibleSkillNames,
  ]);

  return commands.filter(
    (command) =>
      command.command_type !== 'Skill' || allowedSkillNames.has(command.command)
  );
}
