import { describe, it, expect, vi, beforeEach, afterEach, type Mock } from 'vitest';
import { addExtensionFromDeepLink } from './deeplink';

// Mock activateExtension from extension-manager
vi.mock('./extension-manager', () => ({
  activateExtension: vi.fn(),
}));

import { activateExtension } from './extension-manager';

describe('addExtensionFromDeepLink (streamable_http deeplinks)', () => {
  const addExtensionFn = vi.fn().mockResolvedValue(undefined);
  const setView = vi.fn();

  let logSpy: ReturnType<typeof vi.spyOn>;
  let errorSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    vi.clearAllMocks();
    logSpy = vi.spyOn(console, 'log').mockImplementation(() => {});
    errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
  });

  afterEach(() => {
    logSpy.mockRestore();
    errorSpy.mockRestore();
  });

  it('OAuth Trigger Test: calls activateExtension() for streamable_http deeplinks', async () => {
    (activateExtension as unknown as Mock).mockResolvedValueOnce(undefined);

    const deeplink =
      'goose://extension?type=streamable_http&url=' +
      encodeURIComponent('http://localhost:8788/mcp') +
      '&name=Envoy';

    await addExtensionFromDeepLink(deeplink, addExtensionFn, setView);

    expect(activateExtension).toHaveBeenCalledTimes(1);
    const firstArg = (activateExtension as unknown as Mock).mock.calls[0][0];
    expect(firstArg.extensionConfig.type).toBe('streamable_http');
    // On success, modal should not be shown
    expect(setView).not.toHaveBeenCalled();
  });

  it('Fallback Modal Test: opens modal when activateExtension throws', async () => {
    (activateExtension as unknown as Mock).mockRejectedValueOnce(new Error('network error'));

    const deeplink =
      'goose://extension?type=streamable_http&url=' +
      encodeURIComponent('http://localhost:8788/mcp') +
      '&name=Envoy';

    await addExtensionFromDeepLink(deeplink, addExtensionFn, setView);

    expect(activateExtension).toHaveBeenCalledTimes(1);
    expect(setView).toHaveBeenCalledTimes(1);
    expect(setView).toHaveBeenCalledWith('settings', {
      deepLinkConfig: expect.objectContaining({ type: 'streamable_http', name: 'Envoy' }),
      showEnvVars: true,
    });
    expect(console.error).toHaveBeenCalled();
  });

  it('Success Path Test: returns early after successful activation', async () => {
    (activateExtension as unknown as Mock).mockResolvedValueOnce(undefined);

    const deeplink =
      'goose://extension?type=streamable_http&url=' +
      encodeURIComponent('http://localhost:8788/mcp') +
      '&name=Envoy';

    await addExtensionFromDeepLink(deeplink, addExtensionFn, setView);

    // ensure no fallback to modal
    expect(setView).not.toHaveBeenCalled();
    expect(activateExtension).toHaveBeenCalledTimes(1);
    expect(console.log).toHaveBeenCalled();
  });
});

describe('addExtensionFromDeepLink (type-specific behavior)', () => {
  const addExtensionFn = vi.fn().mockResolvedValue(undefined);
  const setView = vi.fn();

  let logSpy: ReturnType<typeof vi.spyOn>;
  let errorSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    vi.clearAllMocks();
    logSpy = vi.spyOn(console, 'log').mockImplementation(() => {});
    errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
  });

  afterEach(() => {
    logSpy.mockRestore();
    errorSpy.mockRestore();
  });

  it('Type-Specific: stdio with env vars opens modal', async () => {
    const deeplink =
      'goose://extension?type=stdio&cmd=npx&name=MyStdIO' + '&env=API_KEY=__REDACTED__';

    await addExtensionFromDeepLink(deeplink, addExtensionFn, setView);

    expect(setView).toHaveBeenCalledTimes(1);
    expect(setView).toHaveBeenCalledWith('settings', {
      deepLinkConfig: expect.objectContaining({ type: 'stdio', name: 'MyStdIO' }),
      showEnvVars: true,
    });
    // No activation attempt for stdio with envs
    expect(activateExtension).not.toHaveBeenCalled();
  });

  it('Type-Specific: streamable_http bypasses env check and activates', async () => {
    (activateExtension as unknown as Mock).mockResolvedValueOnce(undefined);

    const deeplink =
      'goose://extension?type=streamable_http&url=' +
      encodeURIComponent('http://localhost:8788/mcp') +
      '&name=Envoy';

    await addExtensionFromDeepLink(deeplink, addExtensionFn, setView);

    expect(activateExtension).toHaveBeenCalledTimes(1);
    expect(setView).not.toHaveBeenCalled();
  });
});

describe('addExtensionFromDeepLink (logging)', () => {
  const addExtensionFn = vi.fn().mockResolvedValue(undefined);
  const setView = vi.fn();

  let logSpy: ReturnType<typeof vi.spyOn>;
  let errorSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    vi.clearAllMocks();
    logSpy = vi.spyOn(console, 'log').mockImplementation(() => {});
    errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
  });

  afterEach(() => {
    logSpy.mockRestore();
    errorSpy.mockRestore();
  });

  it('logs on activation attempt and error', async () => {
    (activateExtension as unknown as Mock).mockRejectedValueOnce(new Error('auth required'));

    const deeplink =
      'goose://extension?type=streamable_http&url=' +
      encodeURIComponent('http://localhost:8788/mcp') +
      '&name=Envoy';

    await addExtensionFromDeepLink(deeplink, addExtensionFn, setView);

    expect(console.log).toHaveBeenCalled();
    expect(console.error).toHaveBeenCalled();
  });
});
