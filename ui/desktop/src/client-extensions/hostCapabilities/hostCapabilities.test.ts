import { describe, expect, it, vi } from 'vitest';
import { handleHostCapabilityInvoke } from './router';
import { platformPower } from './powers/platform';
import { isHostCapabilityInvokeMessage } from './types';

describe('hostCapabilities', () => {
  it('recognizes grc/host/invoke messages', () => {
    expect(
      isHostCapabilityInvokeMessage({
        type: 'grc/host/invoke',
        capability: 'example',
        method: 'run',
      })
    ).toBe(true);
    expect(isHostCapabilityInvokeMessage({ type: 'grc/mesh/check' })).toBe(false);
  });

  it('rejects invoke when capability is not granted', async () => {
    const postToExtension = vi.fn();
    const handled = await handleHostCapabilityInvoke(
      'demo.addon',
      undefined,
      { type: 'grc/host/invoke', capability: 'example', method: 'run' },
      postToExtension
    );

    expect(handled).toBe(true);
    expect(postToExtension).toHaveBeenCalledWith({
      type: 'grc/host/error',
      capability: 'example',
      method: 'run',
      error: expect.stringContaining('not granted'),
    });
  });

  it('rejects invoke for unknown registered capability id', async () => {
    const postToExtension = vi.fn();
    const handled = await handleHostCapabilityInvoke(
      'demo.addon',
      ['not-a-real-power'],
      { type: 'grc/host/invoke', capability: 'not-a-real-power', method: 'run' },
      postToExtension
    );

    expect(handled).toBe(true);
    expect(postToExtension).toHaveBeenCalledWith({
      type: 'grc/host/error',
      capability: 'not-a-real-power',
      method: 'run',
      error: expect.stringContaining('Unknown host capability'),
    });
  });

  it('returns platform info for granted platform power', async () => {
    vi.stubGlobal('window', {
      electron: { platform: 'darwin', arch: 'arm64' },
    });

    const postToExtension = vi.fn();
    const handled = await handleHostCapabilityInvoke(
      'demo.addon',
      ['platform'],
      { type: 'grc/host/invoke', capability: 'platform', method: 'getInfo' },
      postToExtension
    );

    expect(handled).toBe(true);
    expect(postToExtension).toHaveBeenCalledWith({
      type: 'grc/host/result',
      capability: 'platform',
      method: 'getInfo',
      payload: { platform: 'darwin', arch: 'arm64' },
    });

    vi.unstubAllGlobals();
  });

  it('documents common host powers for plugin authors', () => {
    expect(platformPower.methods).toContain('getInfo');
  });
});
