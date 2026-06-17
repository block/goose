import type { SecurityPreviewSessionMode } from './securityBackendConfig';

export type DesktopUpdateMode = 'enabled' | 'local-preview-disabled' | 'disabled';
export type DesktopUpdateDisabledReason = 'local-preview' | 'updates-disabled';

export interface DesktopUpdateRuntime {
  mode: DesktopUpdateMode;
  disabledReason: DesktopUpdateDisabledReason | null;
  canCheckForUpdates: boolean;
  shouldSetupUpdater: boolean;
}

interface ResolveDesktopUpdateModeOptions {
  previewSessionMode: SecurityPreviewSessionMode;
  updatesEnabled: boolean;
  enableDevUpdates: boolean;
}

export function parseDesktopUpdateMode(value: unknown): DesktopUpdateMode {
  switch (value) {
    case 'enabled':
    case 'local-preview-disabled':
    case 'disabled':
      return value;
    default:
      return 'enabled';
  }
}

export function resolveDesktopUpdateMode(
  options: ResolveDesktopUpdateModeOptions
): DesktopUpdateRuntime {
  if (options.previewSessionMode !== 'standard') {
    return {
      mode: 'local-preview-disabled',
      disabledReason: 'local-preview',
      canCheckForUpdates: false,
      shouldSetupUpdater: false,
    };
  }

  if (options.updatesEnabled || options.enableDevUpdates) {
    return {
      mode: 'enabled',
      disabledReason: null,
      canCheckForUpdates: true,
      shouldSetupUpdater: true,
    };
  }

  return {
    mode: 'disabled',
    disabledReason: 'updates-disabled',
    canCheckForUpdates: false,
    shouldSetupUpdater: false,
  };
}

export function resolveDesktopReleaseRepository(env: Record<string, string | undefined>): {
  owner: string;
  repo: string;
} {
  return {
    owner: env.GITHUB_OWNER?.trim() || 'aaif-goose',
    repo: env.GITHUB_REPO?.trim() || 'goose',
  };
}
