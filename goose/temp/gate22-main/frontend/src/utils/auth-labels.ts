/**
 * Utility functions for displaying authentication type labels and information
 */

/**
 * Get user-friendly label for authentication type
 */
export const getAuthTypeLabel = (authType: string): string => {
  switch (authType) {
    case "no_auth":
      return "No Authentication";
    case "api_key":
      return "API Key";
    case "oauth2":
      return "OAuth 2.0";
    default:
      return authType;
  }
};

/**
 * Get short description for authentication type
 */
export const getAuthTypeDescription = (authType: string): string => {
  switch (authType) {
    case "no_auth":
      return "No authentication required";
    case "api_key":
      return "Use an API key for authentication";
    case "oauth2":
      return "Authenticate via OAuth 2.0 flow";
    default:
      return "";
  }
};

/**
 * Get detailed information/tooltip content for authentication type
 */
export const getAuthTypeDetailedInfo = (authType: string): string => {
  switch (authType) {
    case "no_auth":
      return "This option allows access without any authentication. The MCP server will be accessible without providing any credentials. Use this only for public or non-sensitive services.";
    case "api_key":
      return "Requires providing an API key to authenticate requests. You'll need to obtain an API key from the service provider and enter it when configuring the server. The key will be securely stored and used for all requests.";
    case "oauth2":
      return "Uses OAuth 2.0 for secure authentication. You'll be redirected to the service provider to authorize access. This is the most secure option and allows fine-grained permission control.";
    default:
      return "";
  }
};
