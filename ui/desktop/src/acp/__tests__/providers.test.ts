import { methods } from '@agentclientprotocol/sdk';
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
      connection: {
        agent: {
          request: vi
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
        },
      },
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

    expect(client.connection.agent.request).toHaveBeenCalledTimes(3);
    expect(client.connection.agent.request).toHaveBeenNthCalledWith(
      1,
      methods.agent.session.setConfigOption,
      {
        sessionId: 'session-1',
        configId: 'provider',
        value: 'anthropic',
      }
    );
    expect(client.connection.agent.request).toHaveBeenNthCalledWith(
      2,
      methods.agent.session.setConfigOption,
      {
        sessionId: 'session-1',
        configId: 'model',
        value: 'claude-sonnet-4-5',
      }
    );
    expect(client.connection.agent.request).toHaveBeenNthCalledWith(
      3,
      methods.agent.session.setConfigOption,
      {
        sessionId: 'session-1',
        configId: 'thinking_effort',
        value: 'high',
      }
    );
    expect(applied).toEqual({
      providerId: 'anthropic',
      modelId: 'claude-sonnet-4-5',
    });
  });
});
