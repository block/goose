import { beforeEach, describe, expect, it, vi } from 'vitest';

const apiMocks = vi.hoisted(() => ({
  getProviderModels: vi.fn(),
  readConfig: vi.fn(),
}));

vi.mock('../../../../../../api', () => ({
  getProviderModels: apiMocks.getProviderModels,
  readConfig: apiMocks.readConfig,
}));

import { providerConfigSubmitHandler } from './DefaultSubmitHandler';

describe('providerConfigSubmitHandler', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('marks zero-config providers as configured before checking models', async () => {
    apiMocks.readConfig.mockRejectedValue(new Error('missing'));
    apiMocks.getProviderModels.mockResolvedValue({ data: ['current'] });

    const upsertFn = vi.fn().mockResolvedValue(undefined);
    const removeFn = vi.fn().mockResolvedValue(undefined);

    await providerConfigSubmitHandler(
      upsertFn,
      removeFn,
      {
        name: 'claude-acp',
        metadata: { config_keys: [] },
      },
      {}
    );

    expect(upsertFn).toHaveBeenCalledWith('claude-acp_configured', true, false);
    expect(apiMocks.getProviderModels).toHaveBeenCalledWith({
      path: { name: 'claude-acp' },
      throwOnError: true,
    });
    expect(removeFn).not.toHaveBeenCalled();
  });

  it('removes the configured marker when zero-config validation fails', async () => {
    apiMocks.readConfig.mockRejectedValue(new Error('missing'));
    apiMocks.getProviderModels.mockRejectedValue(new Error('boom'));

    const upsertFn = vi.fn().mockResolvedValue(undefined);
    const removeFn = vi.fn().mockResolvedValue(undefined);

    await expect(
      providerConfigSubmitHandler(
        upsertFn,
        removeFn,
        {
          name: 'claude-acp',
          metadata: { config_keys: [] },
        },
        {}
      )
    ).rejects.toThrow('boom');

    expect(upsertFn).toHaveBeenCalledWith('claude-acp_configured', true, false);
    expect(removeFn).toHaveBeenCalledWith('claude-acp_configured', false);
  });

  it('persists the configured marker for providers with only optional defaults', async () => {
    apiMocks.readConfig.mockRejectedValue(new Error('missing'));

    const upsertFn = vi.fn().mockResolvedValue(undefined);
    const removeFn = vi.fn().mockResolvedValue(undefined);

    await providerConfigSubmitHandler(
      upsertFn,
      removeFn,
      {
        name: 'test-provider',
        metadata: {
          config_keys: [
            {
              name: 'TEST_TIMEOUT',
              default: '30',
              required: false,
              secret: false,
            },
          ],
        },
      },
      {}
    );

    expect(upsertFn).toHaveBeenNthCalledWith(1, 'TEST_TIMEOUT', '30', false);
    expect(upsertFn).toHaveBeenNthCalledWith(2, 'test-provider_configured', true, false);
    expect(apiMocks.getProviderModels).not.toHaveBeenCalled();
  });
});
