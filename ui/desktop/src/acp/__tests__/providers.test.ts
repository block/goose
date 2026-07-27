import { beforeEach, describe, expect, it, vi } from 'vitest';
import { getAcpClient } from '../acpConnection';
import { acpListProviderModels, acpSetSessionProviderModel } from '../providers';

vi.mock('../acpConnection', () => ({
  getAcpClient: vi.fn(),
}));

function selectConfigOption(id: string, currentValue: string) {
  return {
    id,
    name: id,
    type: 'select',
    currentValue,
    options: [],
  };
}

describe('ACP providers', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('sets thinking effort after provider and model, then returns the final config response', async () => {
    const client = {
      setSessionConfigOption: vi
        .fn()
        .mockResolvedValueOnce({
          configOptions: [
            selectConfigOption('provider', 'anthropic'),
            selectConfigOption('model', 'provider-default-model'),
          ],
        })
        .mockResolvedValueOnce({
          configOptions: [
            selectConfigOption('provider', 'anthropic'),
            selectConfigOption('model', 'claude-sonnet-4-5'),
          ],
        })
        .mockResolvedValueOnce({
          configOptions: [
            selectConfigOption('provider', 'anthropic'),
            selectConfigOption('model', 'claude-sonnet-4-5'),
            selectConfigOption('thinking_effort', 'high'),
          ],
        }),
    };
    vi.mocked(getAcpClient).mockResolvedValue(
      client as unknown as Awaited<ReturnType<typeof getAcpClient>>
    );

    const applied = await acpSetSessionProviderModel(
      'session-1',
      'anthropic',
      'claude-sonnet-4-5',
      'high'
    );

    expect(client.setSessionConfigOption).toHaveBeenCalledTimes(3);
    expect(client.setSessionConfigOption).toHaveBeenNthCalledWith(1, {
      sessionId: 'session-1',
      configId: 'provider',
      value: 'anthropic',
    });
    expect(client.setSessionConfigOption).toHaveBeenNthCalledWith(2, {
      sessionId: 'session-1',
      configId: 'model',
      value: 'claude-sonnet-4-5',
    });
    expect(client.setSessionConfigOption).toHaveBeenNthCalledWith(3, {
      sessionId: 'session-1',
      configId: 'thinking_effort',
      value: 'high',
    });
    expect(applied).toEqual({
      providerId: 'anthropic',
      modelId: 'claude-sonnet-4-5',
    });
  });

  describe('acpListProviderModels', () => {
    it('unions live supported models with the inventory cache, live ids first', async () => {
      const client = {
        goose: {
          providersList_unstable: vi.fn().mockResolvedValue({
            entries: [
              {
                providerId: 'anthropic',
                models: [
                  { id: 'claude-opus-4-7', name: 'Claude Opus 4.7', contextLimit: 200000, reasoning: true },
                  { id: 'claude-sonnet-4-5', name: 'Claude Sonnet 4.5', contextLimit: 200000 },
                ],
              },
            ],
          }),
          // Live list omits a cached model (sonnet-4-5) and adds one the cache never had (opus-5).
          providersSupportedModelsList_unstable: vi
            .fn()
            .mockResolvedValue({ providerId: 'anthropic', models: ['claude-opus-5', 'claude-opus-4-7'] }),
        },
      };
      vi.mocked(getAcpClient).mockResolvedValue(
        client as unknown as Awaited<ReturnType<typeof getAcpClient>>
      );

      const models = await acpListProviderModels('anthropic');

      // Live ids lead in their advertised order; the live-only model appears with a synthetic
      // entry; cached metadata is grafted onto matching ids; cached-only models are appended.
      expect(models).toEqual([
        { id: 'claude-opus-5', name: 'claude-opus-5' },
        { id: 'claude-opus-4-7', name: 'Claude Opus 4.7', contextLimit: 200000, reasoning: true },
        { id: 'claude-sonnet-4-5', name: 'Claude Sonnet 4.5', contextLimit: 200000 },
      ]);
    });

    it('falls back to the inventory cache when live discovery fails', async () => {
      const inventory = [
        { id: 'claude-sonnet-4-5', name: 'Claude Sonnet 4.5', contextLimit: 200000 },
      ];
      const client = {
        goose: {
          providersList_unstable: vi
            .fn()
            .mockResolvedValue({ entries: [{ providerId: 'anthropic', models: inventory }] }),
          providersSupportedModelsList_unstable: vi
            .fn()
            .mockRejectedValue(new Error('adapter unavailable')),
        },
      };
      vi.mocked(getAcpClient).mockResolvedValue(
        client as unknown as Awaited<ReturnType<typeof getAcpClient>>
      );

      const models = await acpListProviderModels('anthropic');

      expect(models).toEqual(inventory);
    });

    it('falls back to the inventory cache when live discovery stalls past the timeout', async () => {
      vi.useFakeTimers();
      try {
        const inventory = [
          { id: 'claude-sonnet-4-5', name: 'Claude Sonnet 4.5', contextLimit: 200000 },
        ];
        const client = {
          goose: {
            providersList_unstable: vi
              .fn()
              .mockResolvedValue({ entries: [{ providerId: 'anthropic', models: inventory }] }),
            // Never resolves — simulates a provider that accepts the request but hangs.
            providersSupportedModelsList_unstable: vi.fn().mockReturnValue(new Promise(() => {})),
          },
        };
        vi.mocked(getAcpClient).mockResolvedValue(
          client as unknown as Awaited<ReturnType<typeof getAcpClient>>
        );

        const promise = acpListProviderModels('anthropic');
        await vi.runAllTimersAsync();

        await expect(promise).resolves.toEqual(inventory);
      } finally {
        vi.useRealTimers();
      }
    });

    it('unions live models for agent-category providers regardless of id', async () => {
      const client = {
        goose: {
          providersList_unstable: vi.fn().mockResolvedValue({
            entries: [
              {
                providerId: 'claude-acp',
                category: 'agent',
                models: [{ id: 'sonnet', name: 'Sonnet' }],
              },
            ],
          }),
          providersSupportedModelsList_unstable: vi
            .fn()
            .mockResolvedValue({ providerId: 'claude-acp', models: ['opus[1m]', 'sonnet'] }),
        },
      };
      vi.mocked(getAcpClient).mockResolvedValue(
        client as unknown as Awaited<ReturnType<typeof getAcpClient>>
      );

      const models = await acpListProviderModels('claude-acp');

      expect(models).toEqual([
        { id: 'opus[1m]', name: 'opus[1m]' },
        { id: 'sonnet', name: 'Sonnet' },
      ]);
    });

    it('does not union live models for non-allowlisted model providers', async () => {
      const inventory = [{ id: 'gpt-5', name: 'GPT-5', contextLimit: 400000 }];
      const supportedModels = vi.fn();
      const client = {
        goose: {
          providersList_unstable: vi.fn().mockResolvedValue({
            entries: [{ providerId: 'openai', category: 'model', models: inventory }],
          }),
          // OpenAI's raw /models lists embeddings/audio/image models, so it must not be unioned.
          providersSupportedModelsList_unstable: supportedModels,
        },
      };
      vi.mocked(getAcpClient).mockResolvedValue(
        client as unknown as Awaited<ReturnType<typeof getAcpClient>>
      );

      const models = await acpListProviderModels('openai');

      expect(models).toEqual(inventory);
      // Live discovery is skipped entirely for filtered model providers.
      expect(supportedModels).not.toHaveBeenCalled();
    });
  });
});
