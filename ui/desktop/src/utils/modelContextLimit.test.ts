import { describe, expect, it, vi, beforeEach } from 'vitest';
import {
  DEFAULT_CONTEXT_LIMIT,
  fetchResolvedModelContextLimit,
  readConfiguredContextLimit,
  resolveDisplayContextLimit,
} from './modelContextLimit';
import { getProviderModelInfo } from '../api';
import { acpReadConfig } from '../acp/config';

vi.mock('../api', () => ({
  getProviderModelInfo: vi.fn(),
}));

vi.mock('../acp/config', () => ({
  acpReadConfig: vi.fn(),
}));

describe('modelContextLimit', () => {
  beforeEach(() => {
    vi.mocked(getProviderModelInfo).mockReset();
    vi.mocked(acpReadConfig).mockReset();
  });

  it('returns provider-resolved limit from model-info', async () => {
    vi.mocked(getProviderModelInfo).mockResolvedValue({
      data: { context_limit: 1_000_000, name: 'glm-5.2' },
    } as Awaited<ReturnType<typeof getProviderModelInfo>>);

    await expect(fetchResolvedModelContextLimit('opencode_go', 'glm-5.2')).resolves.toBe(
      1_000_000
    );
  });

  it('falls back to GOOSE_CONTEXT_LIMIT when model-info is unavailable', async () => {
    vi.mocked(getProviderModelInfo).mockRejectedValue(new Error('not configured'));
    vi.mocked(acpReadConfig).mockResolvedValue(500_000);

    await expect(resolveDisplayContextLimit('custom', 'MiniMax-M3')).resolves.toBe(500_000);
  });

  it('uses default when nothing is configured', async () => {
    vi.mocked(getProviderModelInfo).mockRejectedValue(new Error('not configured'));
    vi.mocked(acpReadConfig).mockResolvedValue(null);

    await expect(resolveDisplayContextLimit('custom', 'unknown-model')).resolves.toBe(
      DEFAULT_CONTEXT_LIMIT
    );
  });

  it('readConfiguredContextLimit ignores invalid values', async () => {
    vi.mocked(acpReadConfig).mockResolvedValue(0);
    await expect(readConfiguredContextLimit()).resolves.toBeNull();
  });
});
