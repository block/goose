export const getApiUrl = (endpoint: string): string => {
  const gooseApiHost = String(window.appConfig.get('GOOSE_API_HOST') || '');
  const cleanEndpoint = endpoint.startsWith('/') ? endpoint : `/${endpoint}`;
  return `${gooseApiHost}${cleanEndpoint}`;
};
