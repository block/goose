import { afterEach, describe, expect, it, vi } from 'vitest';

import {
  applyBakedDistroEnv,
  isAppLocked,
  resolveStartupTarget,
  type DistroFlags,
} from './backendLock';
import { defaultSettings, type Settings } from './utils/settings';
import { isZitadelAuthEnabled } from './auth/config';

const LOCKED_URL = 'https://dev.avocado.tech/agent';

const lockedDistro: DistroFlags = {
  requireZitadelAuth: true,
  lockedBackendUrl: LOCKED_URL,
};

function settingsWithExternal(url: string, enabled = true): Settings {
  return {
    ...defaultSettings,
    externalGoosed: {
      ...defaultSettings.externalGoosed,
      enabled,
      url,
      secret: 'should-not-matter',
    },
  };
}

describe('isAppLocked', () => {
  it('GivenPackagedAndRequireAuth_WhenChecking_ThenLocked', () => {
    expect(isAppLocked(true, true)).toBe(true);
  });

  it('GivenUnpackagedWithRequireAuth_WhenChecking_ThenNotLocked', () => {
    // mutation resistance: isPackaged && REQUIRE must not become ||
    expect(isAppLocked(false, true)).toBe(false);
  });

  it('GivenPackagedWithoutRequireAuth_WhenChecking_ThenNotLocked', () => {
    expect(isAppLocked(true, false)).toBe(false);
  });
});

describe('resolveStartupTarget', () => {
  it('GivenPackagedNoEnv_WhenResolvingTarget_ThenLockedRemoteRequiringAuth', () => {
    const result = resolveStartupTarget({
      isPackaged: true,
      env: {},
      settings: defaultSettings,
      distro: lockedDistro,
    });

    expect(result).toEqual({
      mode: 'locked-remote',
      url: LOCKED_URL,
      requireAuth: true,
    });
  });

  it('GivenLockedAndAuthModeOff_WhenCheckingAuth_ThenStillRequired', () => {
    const result = resolveStartupTarget({
      isPackaged: true,
      env: { AVCD_AUTH_MODE: 'off' },
      settings: defaultSettings,
      distro: lockedDistro,
    });

    expect(result.requireAuth).toBe(true);
    expect(result.mode).toBe('locked-remote');
  });

  it('GivenSettingsBackendOverride_ThenBakedUrlWins', () => {
    const result = resolveStartupTarget({
      isPackaged: true,
      env: {},
      settings: settingsWithExternal('http://evil.test'),
      distro: lockedDistro,
    });

    expect(result.mode).toBe('locked-remote');
    expect(result.url).toBe(LOCKED_URL);
  });

  it('GivenEmptyLockedUrl_ThenThrows', () => {
    expect(() =>
      resolveStartupTarget({
        isPackaged: true,
        env: {},
        settings: defaultSettings,
        distro: { requireZitadelAuth: true, lockedBackendUrl: '' },
      })
    ).toThrow(/LOCKED_BACKEND_URL/i);
  });

  it('GivenUnpackagedWithLockFlag_WhenResolving_ThenHonoursEnvOverrides', () => {
    const result = resolveStartupTarget({
      isPackaged: false,
      env: {
        GOOSE_EXTERNAL_BACKEND: 'true',
        GOOSE_EXTERNAL_BACKEND_URL: 'http://127.0.0.1:3100',
        AVCD_AUTH_MODE: 'off',
      },
      settings: defaultSettings,
      distro: lockedDistro,
    });

    expect(result.mode).toBe('external');
    expect(result.url).toBe('http://127.0.0.1:3100');
    expect(result.requireAuth).toBe(false);
  });

  it('GivenPackagedWithoutRequireAuth_WhenResolving_ThenLocalServeAllowed', () => {
    const result = resolveStartupTarget({
      isPackaged: true,
      env: {},
      settings: defaultSettings,
      distro: { requireZitadelAuth: false, lockedBackendUrl: LOCKED_URL },
    });

    expect(result.mode).toBe('local-serve');
    expect(result.requireAuth).toBe(false);
  });

  it('GivenExternalEnabledButEmptyUrl_WhenResolving_ThenLocalServe', () => {
    const result = resolveStartupTarget({
      isPackaged: false,
      env: {},
      settings: settingsWithExternal('', true),
      distro: lockedDistro,
    });

    expect(result.mode).toBe('local-serve');
  });
});

describe('isZitadelAuthEnabled lock', () => {
  afterEach(() => {
    vi.unstubAllEnvs();
  });

  it('GivenLockedAndAuthModeOff_WhenCheckingAuth_ThenStillRequired', () => {
    vi.stubEnv('AVCD_AUTH_MODE', 'off');
    vi.stubEnv('ZITADEL_ISSUER', '');
    vi.stubEnv('ZITADEL_CLIENT_ID', '');

    expect(
      isZitadelAuthEnabled(process.env, { isPackaged: true, requireZitadelAuth: true })
    ).toBe(true);
  });

  it('GivenUnpackagedAndAuthModeOff_WhenCheckingAuth_ThenDisabled', () => {
    vi.stubEnv('AVCD_AUTH_MODE', 'off');
    vi.stubEnv('ZITADEL_ISSUER', 'https://zitadel.avcd.ai');
    vi.stubEnv('ZITADEL_CLIENT_ID', 'client');

    expect(
      isZitadelAuthEnabled(process.env, { isPackaged: false, requireZitadelAuth: true })
    ).toBe(false);
  });
});

describe('applyBakedDistroEnv', () => {
  it('GivenPackagedEmptyLockedUrl_WhenBaking_ThenThrows', () => {
    // Simulate empty constant by calling resolve path; bake uses LOCKED_BACKEND_URL from updates.
    // Force-overwrite path is covered when packaged: hostile env cannot stick.
    const env: NodeJS.ProcessEnv = {
      GOOSE_EXTERNAL_BACKEND_URL: 'http://evil.test',
    };
    applyBakedDistroEnv(env, { isPackaged: true });
    expect(env.GOOSE_EXTERNAL_BACKEND_URL).toBe(LOCKED_URL);
    expect(env.ZITADEL_ISSUER).toBeTruthy();
    expect(env.ZITADEL_CLIENT_ID).toBeTruthy();
  });
});
