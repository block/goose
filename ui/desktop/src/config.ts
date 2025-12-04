export const getApiUrl = (endpoint: string): string => {
  const baseUrl = String(window.appConfig.get('GOOSE_API_HOST') || '');
  const cleanEndpoint = endpoint.startsWith('/') ? endpoint : `/${endpoint}`;
  return `${baseUrl}${cleanEndpoint}`;
};
