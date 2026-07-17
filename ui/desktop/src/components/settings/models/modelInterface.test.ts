import { beforeEach, describe, expect, it, vi } from 'vitest';
import { acpListProviderModels } from '../../../acp/providers';
import type { ProviderDetails } from '../../../types/providers';
import { fetchModelCapabilities, fetchModelsForProviders } from './modelInterface';

vi.mock('../../../acp/providers', () => ({
  acpListProviderDetails: vi.fn(),
  acpListProviderModels: vi.fn(),
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

  it('uses predefined model metadata when inventory has no matching model', async () => {
    vi.mocked(acpListProviderModels).mockResolvedValue([]);

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
