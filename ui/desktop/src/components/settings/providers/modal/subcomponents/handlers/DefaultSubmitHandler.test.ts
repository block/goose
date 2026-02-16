import { beforeEach, describe, expect, it, vi } from 'vitest';

import { providerConfigSubmitHandler } from './DefaultSubmitHandler';
import { getProviderModels, readConfig } from '../../../../../../api';

vi.mock('../../../../../../api', () => ({
  getProviderModels: vi.fn(),
  readConfig: vi.fn(),
}));

describe('providerConfigSubmitHandler', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('keeps existing OPENAI_HOST when saving only another field', async () => {
    const upsertFn = vi.fn().mockResolvedValue(undefined);
    vi.mocked(getProviderModels).mockResolvedValue({} as never);

    vi.mocked(readConfig).mockImplementation(async ({ body }) => {
      if (body.key === 'OPENAI_HOST') {
        return { data: 'https://proxy.example.com/v1' } as never;
      }
      if (body.key === 'OPENAI_BASE_PATH') {
        return { data: 'v1/chat/completions' } as never;
      }
      return { data: null } as never;
    });

    await providerConfigSubmitHandler(
      upsertFn,
      {
        name: 'openai',
        metadata: {
          config_keys: [
            { name: 'OPENAI_API_KEY', required: false, secret: true },
            {
              name: 'OPENAI_HOST',
              required: true,
              secret: false,
              default: 'https://api.openai.com',
            },
            {
              name: 'OPENAI_BASE_PATH',
              required: true,
              secret: false,
              default: 'v1/chat/completions',
            },
          ],
        },
      },
      {
        OPENAI_API_KEY: 'new-key',
      }
    );

    expect(upsertFn).toHaveBeenCalledWith('OPENAI_API_KEY', 'new-key', true);
    expect(upsertFn).toHaveBeenCalledWith('OPENAI_HOST', 'https://proxy.example.com/v1', false);
    expect(upsertFn).not.toHaveBeenCalledWith('OPENAI_HOST', 'https://api.openai.com', false);
  });
});
