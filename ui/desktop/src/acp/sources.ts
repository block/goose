import type { SourceEntry, SourceType } from '@aaif/goose-sdk';
import { getAcpClient } from './acpConnection';

const SKILL_SOURCE_TYPES: SourceType[] = ['skill', 'builtinSkill'];
const inFlightSkillSourceLoads = new Map<string, Promise<SourceEntry[]>>();

const SKILL_NAME_PATTERN = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;

export function validateSkillName(name: string): string | null {
  const trimmed = name.trim();
  if (!trimmed) {
    return 'Skill name is required';
  }
  if (trimmed.length > 64) {
    return 'Skill name must be at most 64 characters';
  }
  if (!SKILL_NAME_PATTERN.test(trimmed)) {
    return 'Use lowercase letters, digits, and hyphens only (no leading/trailing hyphen)';
  }
  return null;
}

export type CreateSkillSourceParams = {
  name: string;
  description: string;
  content: string;
  projectDir: string;
  global?: boolean;
};

export async function createSkillSource(params: CreateSkillSourceParams): Promise<SourceEntry> {
  const nameError = validateSkillName(params.name);
  if (nameError) {
    throw new Error(nameError);
  }
  if (!params.description.trim()) {
    throw new Error('Description is required');
  }
  if (!params.content.trim()) {
    throw new Error('Content is required');
  }

  const client = await getAcpClient();
  const response = await client.goose.sourcesCreate_unstable({
    type: 'skill',
    name: params.name.trim(),
    description: params.description.trim(),
    content: params.content,
    target: params.global
      ? { scope: 'global' }
      : { scope: 'projectDir', projectDir: params.projectDir },
  });

  return response.source;
}

export async function listSkillSources(projectDir: string): Promise<SourceEntry[]> {
  const inFlightLoad = inFlightSkillSourceLoads.get(projectDir);
  if (inFlightLoad) {
    return inFlightLoad;
  }

  const load = loadSkillSources(projectDir);
  inFlightSkillSourceLoads.set(projectDir, load);

  try {
    return await load;
  } finally {
    if (inFlightSkillSourceLoads.get(projectDir) === load) {
      inFlightSkillSourceLoads.delete(projectDir);
    }
  }
}

async function loadSkillSources(projectDir: string): Promise<SourceEntry[]> {
  const client = await getAcpClient();
  const responses = await Promise.all(
    SKILL_SOURCE_TYPES.map((type) =>
      client.goose.sourcesList_unstable({
        type,
        projectDir,
      })
    )
  );

  return responses
    .flatMap((response) => response.sources)
    .sort(
      (a, b) =>
        a.name.localeCompare(b.name, undefined, { sensitivity: 'base' }) ||
        a.path.localeCompare(b.path)
    );
}
