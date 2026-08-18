import { beforeEach, describe, expect, it, vi } from 'vitest';

const sourcesCreate_unstable = vi.fn();
const sourcesList_unstable = vi.fn();

vi.mock('../acpConnection', () => ({
  getAcpClient: vi.fn(async () => ({
    goose: {
      sourcesCreate_unstable,
      sourcesList_unstable,
    },
  })),
}));

import { createSkillSource, validateSkillName } from '../sources';

describe('validateSkillName', () => {
  it('accepts kebab-case names', () => {
    expect(validateSkillName('code-review')).toBeNull();
    expect(validateSkillName('e2e-smoke-test')).toBeNull();
  });

  it('rejects invalid names', () => {
    expect(validateSkillName('')).not.toBeNull();
    expect(validateSkillName('-bad')).not.toBeNull();
    expect(validateSkillName('Bad_Name')).not.toBeNull();
    expect(validateSkillName('a'.repeat(65))).not.toBeNull();
  });
});

describe('createSkillSource', () => {
  beforeEach(() => {
    sourcesCreate_unstable.mockReset();
    sourcesList_unstable.mockReset();
  });

  it('calls ACP sourcesCreate with projectDir target by default', async () => {
    sourcesCreate_unstable.mockResolvedValue({
      source: {
        type: 'skill',
        name: 'hello-world',
        description: 'Greets the user',
        content: '# Hello',
        path: '/workspace/.agents/skills/hello-world',
        global: false,
        writable: true,
        supportingFiles: [],
        properties: {},
      },
    });

    const source = await createSkillSource({
      name: 'hello-world',
      description: 'Greets the user',
      content: '# Hello\n\nSay hi.',
      projectDir: '/workspace',
    });

    expect(sourcesCreate_unstable).toHaveBeenCalledWith({
      type: 'skill',
      name: 'hello-world',
      description: 'Greets the user',
      content: '# Hello\n\nSay hi.',
      target: { scope: 'projectDir', projectDir: '/workspace' },
    });
    expect(source.path).toContain('.agents/skills/hello-world');
  });

  it('uses global scope when requested', async () => {
    sourcesCreate_unstable.mockResolvedValue({
      source: {
        type: 'skill',
        name: 'global-skill',
        description: 'Global',
        content: '# Global',
        path: '/home/goose/.agents/skills/global-skill',
        global: true,
        writable: true,
        supportingFiles: [],
        properties: {},
      },
    });

    await createSkillSource({
      name: 'global-skill',
      description: 'Global',
      content: '# Global',
      projectDir: '/workspace',
      global: true,
    });

    expect(sourcesCreate_unstable).toHaveBeenCalledWith(
      expect.objectContaining({
        target: { scope: 'global' },
      })
    );
  });

  it('rejects invalid names before calling ACP', async () => {
    await expect(
      createSkillSource({
        name: 'Invalid Name',
        description: 'x',
        content: 'y',
        projectDir: '/workspace',
      })
    ).rejects.toThrow(/lowercase/);
    expect(sourcesCreate_unstable).not.toHaveBeenCalled();
  });
});
