// Helper to construct API endpoints
export const getApiUrl = (endpoint: string): string => {
  // GOOSE_API_HOST now contains the full base URL (e.g., http://127.0.0.1:3000)
  const baseUrl = String(window.appConfig.get('GOOSE_API_HOST') || '');
  const cleanEndpoint = endpoint.startsWith('/') ? endpoint : `/${endpoint}`;
  return `${baseUrl}${cleanEndpoint}`;
};
