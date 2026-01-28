// Track the current working dir for this window (updated when user changes it)
let currentWorkingDir: string | null = null;

export const setCurrentWorkingDir = (dir: string): void => {
  currentWorkingDir = dir;
};

export const getInitialWorkingDir = (): string => {
  // Use the current dir if set, otherwise fall back to initial config
  return currentWorkingDir ?? (window.appConfig?.get('GOOSE_WORKING_DIR') as string) ?? '';
};
