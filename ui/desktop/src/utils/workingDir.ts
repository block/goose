export const getInitialWorkingDir = (): string => {
  return (window.appConfig?.get('GOOSE_WORKING_DIR') as string) || '';
};
