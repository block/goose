export const getInitialWorkingDir = (): string => {
  // Fall back to initial config from app startup
  return (window.appConfig?.get('GOOSE_WORKING_DIR') as string) ?? '';
};

/**
 * Resolve the working directory for a new chat in the current window.
 *
 * GOOSE_WORKING_DIR is fixed when the window is created, so it goes stale when
 * the user switches to an external backend (or changes the configured remote
 * directory) afterwards. The configured remote directory is only applied when
 * the window is actually bound to an external backend (fixed at window creation
 * via the gooseServeLeases) and that backend still matches the current
 * settings; otherwise the remote path would be sent to the local (or a
 * different remote) server, where it fails the cwd existence validation.
 */
export const getEffectiveWorkingDir = async (): Promise<string> => {
  const initial = getInitialWorkingDir();
  const boundUrl = window.appConfig?.get('GOOSE_EXTERNAL_BACKEND_URL') as string | undefined;
  if (window.appConfig?.get('GOOSE_EXTERNAL_BACKEND') !== true || !boundUrl) {
    return initial;
  }
  try {
    const external = await window.electron.getSetting('externalGoosed');
    const remote = external?.workingDir?.trim();
    if (external?.enabled && remote && normalizeUrl(boundUrl) === normalizeUrl(external.url)) {
      return remote;
    }
  } catch {
    // Settings unavailable; fall back to the remembered directory.
  }
  return initial;
};

const normalizeUrl = (url: string): string => url.trim().replace(/\/+$/, '');

export const resolveWorkingDir = (
  externalWorkingDir: string | undefined,
  requestedWorkingDir: string | undefined,
  homeDir: string
): string => externalWorkingDir?.trim() || requestedWorkingDir || homeDir;
