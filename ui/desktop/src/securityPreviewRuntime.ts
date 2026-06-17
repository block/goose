export type SecurityPreviewSessionMode =
  | 'standard'
  | 'repo-preview'
  | 'packaged-preview-explicit'
  | 'packaged-preview-fallback';

const SUPPORTED_SCRIPT_COMMAND = './scripts/start-security-packaged-preview.sh';
const SUPPORTED_PNPM_COMMAND = 'pnpm --dir ui/desktop run start:packaged-preview';

function getAppConfigString(key: string): string | null {
  const value = window.appConfig?.get(key);
  return typeof value === 'string' && value.trim() ? value.trim() : null;
}

function parseSecurityPreviewSessionMode(value: string | null): SecurityPreviewSessionMode {
  switch (value) {
    case 'repo-preview':
    case 'packaged-preview-explicit':
    case 'packaged-preview-fallback':
      return value;
    default:
      return 'standard';
  }
}

export function getSecurityPreviewLaunchInfo(): {
  mode: SecurityPreviewSessionMode;
  isPackagedLocalPreview: boolean;
  isFallbackSession: boolean;
  isSupportedEntry: boolean;
  supportedScriptCommand: string;
  supportedPnpmCommand: string;
} {
  const mode = parseSecurityPreviewSessionMode(getAppConfigString('SECURITY_PREVIEW_SESSION_MODE'));

  return {
    mode,
    isPackagedLocalPreview:
      mode === 'packaged-preview-explicit' || mode === 'packaged-preview-fallback',
    isFallbackSession: mode === 'packaged-preview-fallback',
    isSupportedEntry: mode !== 'packaged-preview-fallback',
    supportedScriptCommand: SUPPORTED_SCRIPT_COMMAND,
    supportedPnpmCommand: SUPPORTED_PNPM_COMMAND,
  };
}
