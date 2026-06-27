import { beforeEach, describe, expect, it, vi } from 'vitest';
import { getAcpClient } from '../acpConnection';
import { acpSetSessionProviderModel } from '../providers';

vi.mock('../acpConnection', () => ({
  getAcpClient: vi.fn(),
}));

function selectConfigOption(id: string, currentValue: string) {
  return {
    id,
    name: id,
    kind: {
      type: 'select',
      currentValue,
      options: [],
    },
  };
}

describe('ACP providers', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('returns provider and model from the final model config response', async () => {
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
        }),
    };
    vi.mocked(getAcpClient).mockResolvedValue(
      client as unknown as Awaited<ReturnType<typeof getAcpClient>>
    );

    const applied = await acpSetSessionProviderModel(
      'session-1',
      'anthropic',
      'claude-sonnet-4-5'
    );

    expect(client.setSessionConfigOption).toHaveBeenCalledTimes(2);
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
    expect(applied).toEqual({
      providerId: 'anthropic',
      modelId: 'claude-sonnet-4-5',
    });
  });
});
