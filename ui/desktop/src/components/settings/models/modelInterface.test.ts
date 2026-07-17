import { beforeEach, describe, expect, it, vi } from 'vitest';
import { acpListProviderModels } from '../../../acp/providers';
import { fetchModelCapabilities } from './modelInterface';

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
