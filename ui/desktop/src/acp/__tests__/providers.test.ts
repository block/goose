import { beforeEach, describe, expect, it, vi } from 'vitest';
import { getAcpClient } from '../acpConnection';
import {
  acpGetProviderDetails,
  acpListProviderDetails,
  acpRefreshProviderDetails,
  acpSetSessionProviderModel,
} from '../providers';

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

  it('rechecks an uninstalled ACP adapter without trying to start it', async () => {
    const entry = providerEntry({ configured: false });
    const client = {
      goose: {
        providersList_unstable: vi.fn().mockResolvedValue({ entries: [entry] }),
        providersReadinessCheck_unstable: vi.fn(),
        providersInventoryRefresh_unstable: vi.fn(),
      },
    };
    vi.mocked(getAcpClient).mockResolvedValue(
      client as unknown as Awaited<ReturnType<typeof getAcpClient>>
    );

    const result = await acpRefreshProviderDetails('claude-acp');

    expect(result.provider.is_configured).toBe(false);
    expect(result.connectionChecked).toBe(false);
    expect(client.goose.providersInventoryRefresh_unstable).not.toHaveBeenCalled();
    expect(client.goose.providersReadinessCheck_unstable).not.toHaveBeenCalled();
  });

  it('hides deprecated providers from setup lists but still resolves them directly', async () => {
    const visible = providerEntry();
    const hidden = providerEntry({
      providerId: 'claude_code',
      visibleInSetup: false,
      deprecated: true,
      replacement: 'claude-acp',
    });
    const client = {
      goose: {
        providersList_unstable: vi
          .fn()
          .mockImplementation(({ providerIds }: { providerIds?: string[] }) => ({
            entries: providerIds?.length ? [hidden] : [visible, hidden],
          })),
      },
    };
    vi.mocked(getAcpClient).mockResolvedValue(
      client as unknown as Awaited<ReturnType<typeof getAcpClient>>
    );

    expect(await acpListProviderDetails()).toHaveLength(1);
    expect((await acpGetProviderDetails('claude_code')).replacement).toBe('claude-acp');
  });

  it('probes an installed ACP adapter and returns its refreshed models', async () => {
    const installed = providerEntry({ configured: true, refreshing: false });
    const refreshed = providerEntry({
      configured: true,
      refreshing: false,
      models: [{ id: 'claude-sonnet', name: 'Claude Sonnet', recommended: true }],
    });
    const client = {
      goose: {
        providersList_unstable: vi
          .fn()
          .mockResolvedValueOnce({ entries: [installed] })
          .mockResolvedValueOnce({ entries: [refreshed] }),
        providersReadinessCheck_unstable: vi.fn().mockResolvedValue({
          providerId: 'claude-acp',
          ready: true,
        }),
        providersInventoryRefresh_unstable: vi.fn().mockResolvedValue({
          started: ['claude-acp'],
          skipped: [],
        }),
      },
    };
    vi.mocked(getAcpClient).mockResolvedValue(
      client as unknown as Awaited<ReturnType<typeof getAcpClient>>
    );

    const result = await acpRefreshProviderDetails('claude-acp');

    expect(result.connectionChecked).toBe(true);
    expect(result.provider.metadata.known_models).toEqual([
      { name: 'claude-sonnet', context_limit: 0, reasoning: undefined },
    ]);
  });

  it('surfaces an ACP authentication failure without using model refresh as readiness', async () => {
    const installed = providerEntry({ configured: true });
    const client = {
      goose: {
        providersList_unstable: vi.fn().mockResolvedValue({ entries: [installed] }),
        providersReadinessCheck_unstable: vi.fn().mockResolvedValue({
          providerId: 'claude-acp',
          ready: false,
          error: 'OAuth session expired',
        }),
        providersInventoryRefresh_unstable: vi.fn(),
      },
    };
    vi.mocked(getAcpClient).mockResolvedValue(
      client as unknown as Awaited<ReturnType<typeof getAcpClient>>
    );

    const result = await acpRefreshProviderDetails('claude-acp');

    expect(result.connectionChecked).toBe(true);
    expect(result.readinessError).toBe('OAuth session expired');
    expect(client.goose.providersInventoryRefresh_unstable).not.toHaveBeenCalled();
  });
});

function providerEntry(overrides: Record<string, unknown> = {}) {
  return {
    providerId: 'claude-acp',
    providerName: 'Claude Code',
    description: 'Use Claude Code through ACP',
    defaultModel: 'current',
    configured: true,
    providerType: 'Builtin',
    category: 'agent',
    visibleInSetup: true,
    deprecated: false,
    configKeys: [],
    setupSteps: [],
    supportsRefresh: true,
    refreshing: false,
    models: [],
    stale: false,
    ...overrides,
  };
}
