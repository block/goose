import { beforeEach, describe, expect, it, vi } from 'vitest';
import { acpGetProviderModelCapabilities, acpListProviderModels } from '../../../acp/providers';
import type { ProviderDetails } from '../../../types/providers';
import { fetchModelCapabilities, fetchModelsForProviders } from './modelInterface';

vi.mock('../../../acp/providers', () => ({
  acpListProviderDetails: vi.fn(),
  acpListProviderModels: vi.fn(),
  acpGetProviderModelCapabilities: vi.fn(),
}));

vi.mock('../../../acp/local-inference', () => ({
  listLocalModels: vi.fn(),
}));

describe('fetchModelCapabilities', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('reads reasoning mode support from provider inventory', async () => {
    vi.mocked(acpListProviderModels).mockResolvedValue([
      {
        id: 'gpt-5.6-sol',
        name: 'GPT-5.6 Sol',
        reasoning: true,
        supportsReasoningMode: true,
      },
    ]);

    await expect(fetchModelCapabilities('openai', 'gpt-5.6-sol')).resolves.toEqual({
      reasoning: true,
      supportsReasoningMode: true,
    });
  });

  it('resolves the configured provider route when inventory has no matching model', async () => {
    vi.mocked(acpListProviderModels).mockResolvedValue([]);
    vi.mocked(acpGetProviderModelCapabilities).mockResolvedValue({
      supportsReasoningMode: true,
    });

    await expect(
      fetchModelCapabilities('openai', 'gpt-5.6-sol', {
        reasoning: false,
        supportsReasoningMode: true,
      })
    ).resolves.toEqual({
      reasoning: false,
      supportsReasoningMode: true,
    });
  });

  it('lets an explicit custom chat route override model-name fallback', async () => {
    vi.mocked(acpListProviderModels).mockResolvedValue([]);
    vi.mocked(acpGetProviderModelCapabilities).mockResolvedValue({
      supportsReasoningMode: false,
    });

    await expect(
      fetchModelCapabilities('openai', 'gpt-5.6-custom', { supportsReasoningMode: true })
    ).resolves.toEqual({
      reasoning: null,
      supportsReasoningMode: false,
    });
  });

  it('resolves the route when matching inventory metadata is unknown', async () => {
    vi.mocked(acpListProviderModels).mockResolvedValue([
      {
        id: 'gpt-5.6-custom',
        name: 'GPT-5.6 Custom',
        reasoning: true,
        supportsReasoningMode: null,
      },
    ]);
    vi.mocked(acpGetProviderModelCapabilities).mockResolvedValue({
      supportsReasoningMode: false,
    });

    await expect(fetchModelCapabilities('openai', 'gpt-5.6-custom')).resolves.toEqual({
      reasoning: true,
      supportsReasoningMode: false,
    });
  });
});

describe('fetchModelsForProviders', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('preserves reasoning mode support in the custom provider fallback', async () => {
    vi.mocked(acpListProviderModels).mockRejectedValue(new Error('provider unavailable'));
    const provider = {
      name: 'custom-openai',
      provider_type: 'Custom',
      is_configured: true,
      metadata: {
        name: 'custom-openai',
        display_name: 'Custom OpenAI',
        description: '',
        default_model: 'team-prod',
        model_doc_link: '',
        config_keys: [],
        known_models: [
          {
            name: 'team-prod',
            context_limit: 128_000,
            reasoning: true,
            supports_reasoning_mode: true,
          },
        ],
      },
    } satisfies ProviderDetails;

    const [result] = await fetchModelsForProviders([provider]);

    expect(result.models).toEqual([
      expect.objectContaining({
        name: 'team-prod',
        reasoning: true,
        supports_reasoning_mode: true,
      }),
    ]);
    expect(result.error).toBeNull();
    expect(result.warning).toContain('showing configured models');
  });
});
