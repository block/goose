/**
 * E2E-1 — packaged lockdown (binding acceptance).
 * covers AC-1, AC-2, AC-3
 *
 * Expected FAILING until Phase 3/4 (backendLock + main wiring).
 */
import { describe, expect, it } from 'vitest';

import { resolveStartupTarget } from '../backendLock';
import type { Settings } from '../utils/settings';
import { defaultSettings } from '../utils/settings';

const LOCKED_URL = 'https://dev.avocado.tech/agent';

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

describe('E2E: Packaged lockdown - Goal: downloadable app requires login to Avocado', () => {
  it('GivenPackagedNoEnv_WhenResolvingTarget_ThenLockedRemoteRequiringAuth', () => {
    // covers AC-1
    const result = resolveStartupTarget({
      isPackaged: true,
      env: {},
      settings: defaultSettings,
      distro: {
        requireZitadelAuth: true,
        lockedBackendUrl: LOCKED_URL,
      },
    });

    expect(result.mode).toBe('locked-remote');
    expect(result.url).toBe(LOCKED_URL);
    expect(result.requireAuth).toBe(true);
    expect(result.mode).not.toBe('local-serve');
  });

  it('GivenLockedAndAuthModeOff_WhenResolvingTarget_ThenStillRequireAuth', () => {
    // covers AC-2
    const result = resolveStartupTarget({
      isPackaged: true,
      env: { AVCD_AUTH_MODE: 'off' },
      settings: defaultSettings,
      distro: {
        requireZitadelAuth: true,
        lockedBackendUrl: LOCKED_URL,
      },
    });

    expect(result.requireAuth).toBe(true);
    expect(result.mode).toBe('locked-remote');
  });

  it('GivenSettingsBackendOverride_ThenBakedUrlWins', () => {
    // covers AC-3
    const result = resolveStartupTarget({
      isPackaged: true,
      env: {},
      settings: settingsWithExternal('http://evil.test'),
      distro: {
        requireZitadelAuth: true,
        lockedBackendUrl: LOCKED_URL,
      },
    });

    expect(result.mode).toBe('locked-remote');
    expect(result.url).toBe(LOCKED_URL);
    expect(result.url).not.toBe('http://evil.test');
  });

  it('GivenEmptyLockedUrl_ThenThrows', () => {
    // covers AC-3
    expect(() =>
      resolveStartupTarget({
        isPackaged: true,
        env: {},
        settings: defaultSettings,
        distro: {
          requireZitadelAuth: true,
          lockedBackendUrl: '',
        },
      })
    ).toThrow(/LOCKED_BACKEND_URL/i);
  });

  it('GivenAnyPackagedInput_WhenResolvingTarget_ThenNeverLocalServe', () => {
    // covers AC-1 mutation resistance
    const cases = [
      { env: {}, settings: defaultSettings },
      { env: { AVCD_AUTH_MODE: 'off' }, settings: defaultSettings },
      { env: {}, settings: settingsWithExternal('http://127.0.0.1:9999') },
    ];

    for (const c of cases) {
      const result = resolveStartupTarget({
        isPackaged: true,
        env: c.env,
        settings: c.settings,
        distro: {
          requireZitadelAuth: true,
          lockedBackendUrl: LOCKED_URL,
        },
      });
      expect(result.mode).not.toBe('local-serve');
    }
  });
});
