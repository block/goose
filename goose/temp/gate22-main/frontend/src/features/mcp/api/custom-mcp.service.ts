import { createAuthenticatedRequest } from "@/lib/api-client";
import { CONTROL_PLANE_PATH } from "@/config/api.constants";
import { MCPServerPublic } from "../types/mcp.types";

export interface OAuth2DiscoveryResponse {
  authorize_url?: string;
  access_token_url?: string;
  refresh_token_url?: string;
  registration_url?: string;
  token_endpoint_auth_method_supported?: string[];
}

export interface OAuth2DCRResponse {
  token_endpoint_auth_method: string;
  client_id?: string;
  client_secret?: string;
}

export interface AuthConfig {
  type: string;
  location?: string;
  name?: string;
  prefix?: string;
  authorize_url?: string;
  access_token_url?: string;
  refresh_token_url?: string;
  scope?: string;
  client_id?: string;
  client_secret?: string;
  token_endpoint_auth_method?: string;
}

export interface CreateCustomMCPServerRequest {
  name: string;
  url: string;
  description: string;
  logo?: string;
  categories: string[];
  auth_configs: AuthConfig[];
  server_metadata: object;
  operational_account_auth_type: string;
}

export interface OAuth2DiscoveryRequest {
  mcp_server_url: string;
}

export interface OAuth2DCRRequest {
  mcp_server_url: string;
  registration_url: string;
  token_endpoint_auth_method_supported: string[];
}

const API_ENDPOINTS = {
  MCP_SERVERS: `${CONTROL_PLANE_PATH}/mcp-servers`,
  OAUTH2_DISCOVERY: `${CONTROL_PLANE_PATH}/mcp-servers/oauth2-discovery`,
  OAUTH2_DCR: `${CONTROL_PLANE_PATH}/mcp-servers/oauth2-dcr`,
};

export const customMCPService = {
  // OAuth2 Discovery
  discoverOAuth2: async (
    token: string,
    orgId: string | undefined,
    role: string | undefined,
    request: OAuth2DiscoveryRequest,
  ): Promise<OAuth2DiscoveryResponse> => {
    const api = createAuthenticatedRequest(token, orgId, role);
    return api.post<OAuth2DiscoveryResponse>(API_ENDPOINTS.OAUTH2_DISCOVERY, request);
  },

  // OAuth2 Dynamic Client Registration
  registerOAuth2Client: async (
    token: string,
    orgId: string | undefined,
    role: string | undefined,
    request: OAuth2DCRRequest,
  ): Promise<OAuth2DCRResponse> => {
    const api = createAuthenticatedRequest(token, orgId, role);
    return api.post<OAuth2DCRResponse>(API_ENDPOINTS.OAUTH2_DCR, request);
  },

  // Create custom MCP server
  create: async (
    token: string,
    orgId: string | undefined,
    role: string | undefined,
    request: CreateCustomMCPServerRequest,
  ): Promise<MCPServerPublic> => {
    const api = createAuthenticatedRequest(token, orgId, role);
    return api.post<MCPServerPublic>(API_ENDPOINTS.MCP_SERVERS, request);
  },
};
