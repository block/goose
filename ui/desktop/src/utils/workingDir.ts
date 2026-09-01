// GOOSE_WORKING_DIR is fixed when the window is created, so it goes stale when
// the backend is switched in place; the live value comes from the window's
// current backend lease and is refreshed after a switch.
let liveWorkingDir: string | null = null;

export const getInitialWorkingDir = (): string =>
  liveWorkingDir ?? (window.appConfig?.get('GOOSE_WORKING_DIR') as string) ?? '';

export const refreshWorkingDir = async (): Promise<string> => {
  try {
    const workingDir = await window.electron.getWorkingDir();
    if (workingDir) {
      liveWorkingDir = workingDir;
    }
  } catch {
    // Keep the last known directory if the lease cannot be read.
  }
  return getInitialWorkingDir();
};

export const setWorkingDir = (workingDir: string | null): void => {
  liveWorkingDir = workingDir?.trim() ? workingDir : null;
};

export const resolveWorkingDir = (
  externalWorkingDir: string | undefined,
  requestedWorkingDir: string | undefined,
  homeDir: string
): string => externalWorkingDir?.trim() || requestedWorkingDir || homeDir;
