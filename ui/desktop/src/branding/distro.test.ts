import path from 'node:path';
import { describe, expect, it } from 'vitest';

import { loadSecurityDistroDefaults, parseEnvFile } from './distro';

describe('parseEnvFile', () => {
  it('parses dotenv-style content and ignores comments', () => {
    const parsed = parseEnvFile(`
      # comment
      GOOSE_DEFAULT_PROVIDER=openai
      GOOSE_LOCALE=zh-CN

      GOOSE_DEFAULT_MODEL=auto
    `);

    expect(parsed).toEqual({
      GOOSE_DEFAULT_PROVIDER: 'openai',
      GOOSE_LOCALE: 'zh-CN',
      GOOSE_DEFAULT_MODEL: 'auto',
    });
  });
});

describe('loadSecurityDistroDefaults', () => {
  it('loads branding and desktop defaults from distro/security-cn', () => {
    const defaults = loadSecurityDistroDefaults(path.resolve(process.cwd(), '../..'));

    expect(defaults.productName).toBe('Security Goose');
    expect(defaults.productNameZh).toBe('Security Goose 安全工作台');
    expect(defaults.locale).toBe('zh-CN');
    expect(defaults.defaultProvider).toBe('openai');
    expect(defaults.defaultModel).toBe('auto');

    const predefinedModels = JSON.parse(defaults.predefinedModels);
    expect(predefinedModels[0]).toMatchObject({
      name: 'auto',
      provider: 'openai',
      alias: 'Auto',
    });
    expect(predefinedModels).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ name: 'deepseek-v4-flash' }),
        expect.objectContaining({ name: 'deepseek-v4-pro' }),
        expect.objectContaining({ name: 'kimi-k2.6' }),
        expect.objectContaining({ name: 'glm-5.1' }),
      ])
    );
  });
});
