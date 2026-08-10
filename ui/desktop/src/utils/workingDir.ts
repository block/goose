export const getInitialWorkingDir = (): string => {
  // Fall back to initial config from app startup
  return (window.appConfig?.get('GOOSE_WORKING_DIR') as string) ?? '';
};

export const resolveWorkingDir = (
  externalWorkingDir: string | undefined,
  requestedWorkingDir: string | undefined,
  homeDir: string
): string => externalWorkingDir?.trim() || requestedWorkingDir || homeDir;
