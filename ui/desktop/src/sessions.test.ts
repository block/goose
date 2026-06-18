/**
 * @vitest-environment jsdom
 */
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { createSession, ensureSessionProviderAndModelConfigured } from './sessions';

const readConfigMock = vi.fn();
const setConfigProviderMock = vi.fn();
const startAgentMock = vi.fn();

vi.mock('./api', () => ({
  readConfig: (...args: unknown[]) => readConfigMock(...args),
  setConfigProvider: (...args: unknown[]) => setConfigProviderMock(...args),
  startAgent: (...args: unknown[]) => startAgentMock(...args),
}));

vi.mock('./recipe', () => ({
  decodeRecipe: vi.fn(),
}));

vi.mock('./store/extensionOverrides', () => ({
  clearExtensionOverrides: vi.fn(),
  getExtensionConfigsWithOverrides: vi.fn().mockReturnValue([]),
  hasExtensionOverrides: vi.fn().mockReturnValue(false),
}));

vi.mock('./components/settings/models/predefinedModelsUtils', () => ({
  getConfiguredDefaultPredefinedModel: () => ({
    name: 'deepseek-v4-flash',
    provider: 'openai',
    alias: 'DeepSeek V4 Flash',
  }),
}));

describe('ensureSessionProviderAndModelConfigured', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    readConfigMock.mockResolvedValue({ data: null });
    setConfigProviderMock.mockResolvedValue({});
    startAgentMock.mockResolvedValue({ data: { id: 'session-1' } });
    (window as unknown as Record<string, unknown>).appConfig = {
      get: (key: string) => {
        if (key === 'GOOSE_DEFAULT_PROVIDER') return 'openai';
        if (key === 'GOOSE_DEFAULT_MODEL') return 'deepseek-v4-flash';
        return null;
      },
    };
  });

  it('writes the desktop default provider and model when backend config is empty', async () => {
    await ensureSessionProviderAndModelConfigured();

    expect(setConfigProviderMock).toHaveBeenCalledWith({
      body: {
        provider: 'openai',
        model: 'deepseek-v4-flash',
      },
      throwOnError: true,
    });
  });

  it('does not overwrite an already configured provider/model pair', async () => {
    readConfigMock
      .mockResolvedValueOnce({ data: 'openai' })
      .mockResolvedValueOnce({ data: 'glm-5-turbo' });

    await ensureSessionProviderAndModelConfigured();

    expect(setConfigProviderMock).not.toHaveBeenCalled();
  });

  it('ensures provider/model defaults before creating a new session', async () => {
    await createSession('/workspace');

    expect(setConfigProviderMock).toHaveBeenCalledBefore(startAgentMock);
    expect(startAgentMock).toHaveBeenCalledWith({
      body: {
        working_dir: '/workspace',
      },
      throwOnError: true,
    });
  });
});
