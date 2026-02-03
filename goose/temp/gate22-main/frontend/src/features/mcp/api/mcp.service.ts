import { fetcherWithAuth, createAuthenticatedRequest } from "@/lib/api-client";
import {
  MCPServerPublic,
  MCPServerConfigurationPublic,
  MCPServerConfigurationPublicBasic,
  MCPServerConfigurationCreate,
  MCPServerConfigurationUpdate,
  MCPToolPublic,
  PaginationParams,
  PaginationResponse,
  ToolsSyncResult,
} from "../types/mcp.types";
import { CONTROL_PLANE_PATH } from "@/config/api.constants";

const API_ENDPOINTS = {
  SERVERS: `${CONTROL_PLANE_PATH}/mcp-servers`,
  CONFIGURATIONS: `${CONTROL_PLANE_PATH}/mcp-server-configurations`,
  TOOLS: `${CONTROL_PLANE_PATH}/mcp-tools`,
};

export const mcpService = {
  // MCP Tools endpoints
  tools: {
    getByName: async (token: string, toolName: string): Promise<MCPToolPublic> => {
      return fetcherWithAuth<MCPToolPublic>(token)(`${API_ENDPOINTS.TOOLS}/${toolName}`);
    },
  },

  // MCP Servers endpoints
  servers: {
    list: async (
      token: string,
      params?: PaginationParams,
    ): Promise<PaginationResponse<MCPServerPublic>> => {
      return fetcherWithAuth<PaginationResponse<MCPServerPublic>>(token)(API_ENDPOINTS.SERVERS, {
        params: {
          offset: params?.offset?.toString() || "0",
          limit: params?.limit?.toString() || "100",
        },
      });
    },

    getById: async (token: string, serverId: string): Promise<MCPServerPublic> => {
      return fetcherWithAuth<MCPServerPublic>(token)(`${API_ENDPOINTS.SERVERS}/${serverId}`);
    },

    getByName: async (token: string, serverName: string): Promise<MCPServerPublic | undefined> => {
      // This would need a search endpoint on the backend
      // For now, fetch all and filter client-side
      const response = await fetcherWithAuth<PaginationResponse<MCPServerPublic>>(token)(
        API_ENDPOINTS.SERVERS,
        {
          params: {
            offset: "0",
            limit: "100",
          },
        },
      );
      return response.data.find((s) => s.name.toLowerCase() === serverName.toLowerCase());
    },

    syncTools: async (token: string, serverId: string): Promise<ToolsSyncResult> => {
      const api = createAuthenticatedRequest(token);
      return api.post<ToolsSyncResult>(`${API_ENDPOINTS.SERVERS}/${serverId}/refresh-tools`, {});
    },

    update: async (
      token: string,
      serverId: string,
      data: { description?: string; logo?: string },
    ): Promise<MCPServerPublic> => {
      const api = createAuthenticatedRequest(token);
      return api.patch<MCPServerPublic>(`${API_ENDPOINTS.SERVERS}/${serverId}`, data);
    },

    delete: async (token: string, serverId: string): Promise<void> => {
      const api = createAuthenticatedRequest(token);
      return api.delete(`${API_ENDPOINTS.SERVERS}/${serverId}`);
    },
  },

  // MCP Server Configurations endpoints (requires auth)
  configurations: {
    list: async (
      token: string,
      params?: PaginationParams,
    ): Promise<PaginationResponse<MCPServerConfigurationPublicBasic>> => {
      return fetcherWithAuth<PaginationResponse<MCPServerConfigurationPublicBasic>>(token)(
        API_ENDPOINTS.CONFIGURATIONS,
        {
          params: {
            offset: params?.offset?.toString() || "0",
            limit: params?.limit?.toString() || "100",
          },
        },
      );
    },

    getById: async (
      token: string,
      configurationId: string,
    ): Promise<MCPServerConfigurationPublic> => {
      return fetcherWithAuth<MCPServerConfigurationPublic>(token)(
        `${API_ENDPOINTS.CONFIGURATIONS}/${configurationId}`,
      );
    },

    create: async (
      token: string,
      data: MCPServerConfigurationCreate,
    ): Promise<MCPServerConfigurationPublic> => {
      const api = createAuthenticatedRequest(token);
      return api.post<MCPServerConfigurationPublic>(API_ENDPOINTS.CONFIGURATIONS, data);
    },

    update: async (
      token: string,
      configurationId: string,
      data: MCPServerConfigurationUpdate,
    ): Promise<MCPServerConfigurationPublic> => {
      const api = createAuthenticatedRequest(token);
      return api.patch<MCPServerConfigurationPublic>(
        `${API_ENDPOINTS.CONFIGURATIONS}/${configurationId}`,
        data,
      );
    },

    delete: async (token: string, configurationId: string): Promise<void> => {
      const api = createAuthenticatedRequest(token);
      return api.delete(`${API_ENDPOINTS.CONFIGURATIONS}/${configurationId}`);
    },

    listOperational: async (
      token: string,
      params?: PaginationParams,
    ): Promise<PaginationResponse<MCPServerConfigurationPublic>> => {
      return fetcherWithAuth<PaginationResponse<MCPServerConfigurationPublic>>(token)(
        API_ENDPOINTS.CONFIGURATIONS,
        {
          params: {
            connected_account_ownerships: "operational",
            offset: params?.offset?.toString() || "0",
            limit: params?.limit?.toString() || "100",
          },
        },
      );
    },
  },
};
