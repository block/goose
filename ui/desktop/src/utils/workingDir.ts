export const getInitialWorkingDir = (): string => {
  // Fall back to initial config from app startup
  return (window.appConfig?.get('GOOSE_WORKING_DIR') as string) ?? '';
};

/**
 * Resolve the working directory for a new chat in the current window.
 *
 * GOOSE_WORKING_DIR is fixed when the window is created, so it goes stale when
 * the user switches to an external backend (or changes the configured remote
 * directory) afterwards. When an external backend is enabled with a configured
 * remote directory, that directory takes precedence — the local path is
 * meaningless on the remote host and would fail the server's cwd validation.
 */
export const getEffectiveWorkingDir = async (): Promise<string> => {
  const initial = getInitialWorkingDir();
  try {
    const external = await window.electron.getSetting('externalGoosed');
    const remote = external?.workingDir?.trim();
    if (external?.enabled && remote) {
      return remote;
    }
  } catch {
    // Settings unavailable; fall back to the remembered directory.
  }
  return initial;
};

export const resolveWorkingDir = (
  externalWorkingDir: string | undefined,
  requestedWorkingDir: string | undefined,
  homeDir: string
): string => externalWorkingDir?.trim() || requestedWorkingDir || homeDir;
