import { afterEach, describe, expect, it } from 'vitest';

import { getSecurityPreviewLaunchInfo } from './securityPreviewRuntime';

function mockAppConfig(values: Record<string, unknown>) {
  (window as unknown as Record<string, unknown>).appConfig = {
    get: (key: string) => values[key],
    getAll: () => values,
  };
}

describe('securityPreviewRuntime', () => {
  afterEach(() => {
    delete (window as unknown as Record<string, unknown>).appConfig;
  });

  it('marks packaged fallback sessions as unsupported launch entries', () => {
    mockAppConfig({
      SECURITY_PREVIEW_SESSION_MODE: 'packaged-preview-fallback',
    });

    expect(getSecurityPreviewLaunchInfo()).toMatchObject({
      mode: 'packaged-preview-fallback',
      isPackagedLocalPreview: true,
      isFallbackSession: true,
      isSupportedEntry: false,
    });
  });

  it('keeps official packaged preview sessions out of fallback mode', () => {
    mockAppConfig({
      SECURITY_PREVIEW_SESSION_MODE: 'packaged-preview-explicit',
    });

    expect(getSecurityPreviewLaunchInfo()).toMatchObject({
      mode: 'packaged-preview-explicit',
      isPackagedLocalPreview: true,
      isFallbackSession: false,
      isSupportedEntry: true,
    });
  });

  it('treats non-preview sessions as standard desktop launches', () => {
    mockAppConfig({});

    expect(getSecurityPreviewLaunchInfo()).toMatchObject({
      mode: 'standard',
      isPackagedLocalPreview: false,
      isFallbackSession: false,
      isSupportedEntry: true,
    });
  });
});
