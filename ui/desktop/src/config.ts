// Helper to construct API endpoints
export const getApiUrl = (endpoint: string): string => {
  const baseUrl =
    String(window.appConfig.get('GOOSE_API_HOST') || '') +
    ':' +
    String(window.appConfig.get('GOOSE_PORT') || '');
  const cleanEndpoint = endpoint.startsWith('/') ? endpoint : `/${endpoint}`;
  return `${baseUrl}${cleanEndpoint}`;
};
