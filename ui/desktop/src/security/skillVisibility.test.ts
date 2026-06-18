import { describe, expect, it } from 'vitest';
import type { SlashCommand } from '../api';
import {
  filterVisibleSkillCommands,
  SECURITY_GOOSE_VISIBLE_SKILLS_SCOPE,
} from './skillVisibility';

function command(
  commandName: string,
  commandType: SlashCommand['command_type'],
  help = 'help'
): SlashCommand {
  return {
    command: commandName,
    command_type: commandType,
    help,
  };
}

describe('filterVisibleSkillCommands', () => {
  it('keeps all commands when Security Goose scoped visibility is disabled', () => {
    const commands = [
      command('custom-claude-skill', 'Skill'),
      command('goose-doc-guide', 'Skill'),
      command('security-vuln-triage', 'Recipe'),
    ];

    expect(filterVisibleSkillCommands(commands, undefined)).toEqual(commands);
  });

  it('limits visible skills to Goose built-ins and bundled Security Goose skills', () => {
    const commands = [
      command('custom-claude-skill', 'Skill'),
      command('goose-doc-guide', 'Skill'),
      command('alert-triage', 'Skill'),
      command('vuln-triage', 'Skill'),
      command('security-vuln-triage', 'Recipe'),
      command('developer', 'Builtin'),
    ];

    expect(filterVisibleSkillCommands(commands, SECURITY_GOOSE_VISIBLE_SKILLS_SCOPE)).toEqual([
      command('goose-doc-guide', 'Skill'),
      command('alert-triage', 'Skill'),
      command('vuln-triage', 'Skill'),
      command('security-vuln-triage', 'Recipe'),
      command('developer', 'Builtin'),
    ]);
  });

  it('keeps current-project managed local skills visible in curated mode', () => {
    const commands = [
      command('custom-claude-skill', 'Skill'),
      command('local-investigation', 'Skill'),
      command('goose-doc-guide', 'Skill'),
    ];

    expect(
      filterVisibleSkillCommands(commands, SECURITY_GOOSE_VISIBLE_SKILLS_SCOPE, [
        'local-investigation',
      ])
    ).toEqual([command('local-investigation', 'Skill'), command('goose-doc-guide', 'Skill')]);
  });
});
