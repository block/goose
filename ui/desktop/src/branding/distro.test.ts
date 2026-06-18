import path from 'node:path';
import { describe, expect, it } from 'vitest';

import { loadSecurityDistroDefaults, parseEnvFile } from './distro';

describe('parseEnvFile', () => {
  it('parses dotenv-style content and ignores comments', () => {
    const parsed = parseEnvFile(`
      # comment
      GOOSE_DEFAULT_PROVIDER=openai
      GOOSE_LOCALE=zh-CN

      GOOSE_DEFAULT_MODEL=deepseek-v4-flash
    `);

    expect(parsed).toEqual({
      GOOSE_DEFAULT_PROVIDER: 'openai',
      GOOSE_LOCALE: 'zh-CN',
      GOOSE_DEFAULT_MODEL: 'deepseek-v4-flash',
    });
  });
});

describe('loadSecurityDistroDefaults', () => {
  it('loads branding and desktop defaults from distro/security-cn', () => {
    const defaults = loadSecurityDistroDefaults(path.resolve(process.cwd(), '../..'));

    expect(defaults.productName).toBe('收到');
    expect(defaults.productNameZh).toBe('收到');
    expect(defaults.locale).toBe('zh-CN');
    expect(defaults.defaultProvider).toBe('openai');
    expect(defaults.defaultModel).toBe('deepseek-v4-flash');
    expect(defaults.pricingMode).toBe('disabled-token-plan');

    const predefinedModels = JSON.parse(defaults.predefinedModels);
    expect(predefinedModels[0]).toMatchObject({
      name: 'auto',
      provider: 'openai',
      alias: 'Auto',
    });
    expect(predefinedModels.map((model: { name: string }) => model.name)).toEqual([
      'auto',
      'deepseek-v4-flash',
      'deepseek-v4-flash-202605',
      'deepseek-v4-pro',
      'deepseek-v4-pro-202606',
      'glm-5',
      'glm-5-turbo',
      'glm-5.1',
      'kimi-k2.5',
      'kimi-k2.6',
      'minimax-m2.5',
      'minimax-m2.7',
      'minimax-m3',
    ]);
  });
});
