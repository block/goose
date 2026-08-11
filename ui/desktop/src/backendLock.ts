import type { Settings } from './utils/settings';
import {
  BAKED_ZITADEL_CLIENT_ID,
  BAKED_ZITADEL_GOOGLE_IDP_ID,
  BAKED_ZITADEL_ISSUER,
  BAKED_ZITADEL_ORG_ID,
  BAKED_ZITADEL_PROJECT_ID,
  LOCKED_BACKEND_URL,
  REQUIRE_ZITADEL_AUTH,
} from './updates';

export type DistroFlags = {
  requireZitadelAuth: boolean;
  lockedBackendUrl: string;
};

export type StartupTarget = {
  mode: 'locked-remote' | 'external' | 'local-serve';
  url?: string;
  requireAuth: boolean;
};

export type ResolveStartupTargetInput = {
  isPackaged: boolean;
  env: NodeJS.ProcessEnv;
  settings: Settings;
  distro: DistroFlags;
};

/** True only when distro lock flag is on AND the build is packaged. */
export function isAppLocked(isPackaged: boolean, requireZitadelAuth = REQUIRE_ZITADEL_AUTH): boolean {
  return requireZitadelAuth === true && isPackaged === true;
}

function envAuthRequested(env: NodeJS.ProcessEnv): boolean {
  const mode = env.AVCD_AUTH_MODE?.trim().toLowerCase();
  if (mode === 'off' || mode === 'false' || mode === '0') return false;
  if (mode === 'zitadel' || mode === 'on' || mode === 'true' || mode === '1') {
    return Boolean(env.ZITADEL_ISSUER?.trim() && env.ZITADEL_CLIENT_ID?.trim());
  }
  return Boolean(env.ZITADEL_ISSUER?.trim() && env.ZITADEL_CLIENT_ID?.trim());
}

function externalUrlFromEnv(env: NodeJS.ProcessEnv): string | null {
  if (!env.GOOSE_EXTERNAL_BACKEND) {
    return null;
  }
  const configuredUrl = env.GOOSE_EXTERNAL_BACKEND_URL?.trim();
  if (configuredUrl) {
    return configuredUrl;
  }
  return `http://127.0.0.1:${env.GOOSE_PORT || '3000'}`;
}

/**
 * Pure startup target resolution for the Avocado distro lock.
 * main.ts branches on the result; all lock policy lives here for unit tests.
 */
export function resolveStartupTarget(input: ResolveStartupTargetInput): StartupTarget {
  const locked = isAppLocked(input.isPackaged, input.distro.requireZitadelAuth);

  if (locked) {
    const url = (input.distro.lockedBackendUrl ?? '').trim();
    if (!url) {
      throw new Error(
        'LOCKED_BACKEND_URL is empty — refusing to start locked packaged build'
      );
    }
    // Ignore settings.externalGoosed and env backend overrides when locked.
    return {
      mode: 'locked-remote',
      url,
      requireAuth: true,
    };
  }

  const envUrl = externalUrlFromEnv(input.env);
  if (envUrl) {
    return {
      mode: 'external',
      url: envUrl,
      requireAuth: envAuthRequested(input.env),
    };
  }

  if (input.settings.externalGoosed?.enabled && input.settings.externalGoosed.url?.trim()) {
    return {
      mode: 'external',
      url: input.settings.externalGoosed.url.trim(),
      requireAuth: envAuthRequested(input.env),
    };
  }

  return {
    mode: 'local-serve',
    requireAuth: false,
  };
}

function detectIsPackaged(): boolean {
  try {
    // Lazy require keeps this module usable in vitest without a full Electron runtime.
    // eslint-disable-next-line @typescript-eslint/no-require-imports
    const electron = require('electron') as { app?: { isPackaged?: boolean } };
    return Boolean(electron.app?.isPackaged);
  } catch {
    return false;
  }
}

/**
 * Bake distro auth/backend constants into process.env for packaged builds
 * that ship without a .env file. Called from the env-macro hook in main.ts.
 */
export function applyBakedDistroEnv(
  env: NodeJS.ProcessEnv = process.env,
  opts?: { isPackaged?: boolean }
): void {
  if (!REQUIRE_ZITADEL_AUTH) {
    return;
  }

  env.ZITADEL_ISSUER ||= BAKED_ZITADEL_ISSUER;
  env.ZITADEL_CLIENT_ID ||= BAKED_ZITADEL_CLIENT_ID;
  env.ZITADEL_PROJECT_ID ||= BAKED_ZITADEL_PROJECT_ID;
  env.ZITADEL_ORG_ID ||= BAKED_ZITADEL_ORG_ID;
  env.ZITADEL_GOOGLE_IDP_ID ||= BAKED_ZITADEL_GOOGLE_IDP_ID;

  const isPackaged = opts?.isPackaged ?? detectIsPackaged();
  if (!isPackaged) {
    return;
  }

  const lockedUrl = LOCKED_BACKEND_URL.trim();
  if (!lockedUrl) {
    throw new Error(
      'LOCKED_BACKEND_URL is empty — refusing to start locked packaged build'
    );
  }

  // Force the baked gateway URL; do not honour a scrubbed/hostile env override.
  env.GOOSE_EXTERNAL_BACKEND = 'true';
  env.GOOSE_EXTERNAL_BACKEND_URL = lockedUrl;
}

/** Distro flags snapshot for resolveStartupTarget callers. */
export function currentDistroFlags(): DistroFlags {
  return {
    requireZitadelAuth: REQUIRE_ZITADEL_AUTH,
    lockedBackendUrl: LOCKED_BACKEND_URL,
  };
}
