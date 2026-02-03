import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { mcpService } from "../api/mcp.service";
import { MCPServerConfigurationCreate, PaginationParams } from "../types/mcp.types";
import { useMetaInfo } from "@/components/context/metainfo";
import { PERMISSIONS } from "@/lib/rbac/permissions";

// Query keys
export const mcpQueryKeys = {
  all: ["mcp"] as const,
  servers: {
    all: ["mcp", "servers"] as const,
    list: (params?: PaginationParams) => ["mcp", "servers", "list", params] as const,
    detail: (id: string) => ["mcp", "servers", "detail", id] as const,
    byName: (name: string) => ["mcp", "servers", "byName", name] as const,
  },
  configurations: {
    all: ["mcp", "configurations"] as const,
    list: (params?: PaginationParams, authContextKey?: string) =>
      ["mcp", "configurations", "list", params, authContextKey] as const,
    detail: (id: string, authContextKey?: string) =>
      ["mcp", "configurations", "detail", id, authContextKey] as const,
  },
  tools: {
    all: ["mcp", "tools"] as const,
    detail: (name: string) => ["mcp", "tools", "detail", name] as const,
  },
};

// Hook to list all available MCP servers
export function useMCPServers(params?: PaginationParams) {
  const { accessToken } = useMetaInfo();
  return useQuery({
    queryKey: mcpQueryKeys.servers.list(params),
    queryFn: () => mcpService.servers.list(accessToken!, params),
  });
}

// Hook to get a specific MCP server by ID
export function useMCPServer(serverId: string) {
  const { accessToken } = useMetaInfo();

  return useQuery({
    queryKey: mcpQueryKeys.servers.detail(serverId),
    queryFn: () => mcpService.servers.getById(accessToken!, serverId),
    enabled: !!accessToken && !!serverId,
  });
}

// Hook to get a specific MCP server by name
export function useMCPServerByName(serverName: string) {
  const { accessToken } = useMetaInfo();
  return useQuery({
    queryKey: mcpQueryKeys.servers.byName(serverName),
    queryFn: () => mcpService.servers.getByName(accessToken!, serverName),
    enabled: !!serverName,
  });
}

// Hook to list MCP server configurations for the current organization
export function useMCPServerConfigurations(params?: PaginationParams) {
  const { accessToken, checkPermission, activeOrg, activeRole } = useMetaInfo();

  // Create auth context key for cache separation without exposing token
  const authContextKey = activeOrg ? `${activeOrg.orgId}:${activeRole}` : undefined;

  const query = useQuery({
    queryKey: mcpQueryKeys.configurations.list(params, authContextKey),
    queryFn: () => mcpService.configurations.list(accessToken!, params),
    enabled: !!accessToken,
  });

  // Add permission flags for UI decisions
  const canConfigure = checkPermission(PERMISSIONS.MCP_CONFIGURATION_CREATE);
  const canDelete = checkPermission(PERMISSIONS.MCP_CONFIGURATION_DELETE);

  return {
    ...query,
    canConfigure,
    canDelete,
  };
}

// Hook to get a specific MCP server configuration
export function useMCPServerConfiguration(configurationId: string) {
  const { accessToken, activeOrg, activeRole } = useMetaInfo();

  // Create auth context key for cache separation without exposing token
  const authContextKey = activeOrg ? `${activeOrg.orgId}:${activeRole}` : undefined;

  return useQuery({
    queryKey: mcpQueryKeys.configurations.detail(configurationId, authContextKey),
    queryFn: () => mcpService.configurations.getById(accessToken!, configurationId),
    enabled: !!accessToken && !!configurationId,
  });
}

// Hook to create a new MCP server configuration with permission check
export function useCreateMCPServerConfiguration() {
  const { accessToken, checkPermission } = useMetaInfo();
  const queryClient = useQueryClient();

  const canConfigure = checkPermission(PERMISSIONS.MCP_CONFIGURATION_CREATE);

  return useMutation({
    mutationFn: (data: MCPServerConfigurationCreate) => {
      if (!canConfigure) {
        throw new Error("You do not have permission to configure MCP servers");
      }
      return mcpService.configurations.create(accessToken!, data);
    },
    onSuccess: () => {
      // Invalidate configurations list to refetch
      queryClient.invalidateQueries({
        queryKey: mcpQueryKeys.configurations.all,
      });
    },
  });
}

// Hook to delete an MCP server configuration with permission check
export function useDeleteMCPServerConfiguration() {
  const { accessToken, checkPermission } = useMetaInfo();
  const queryClient = useQueryClient();

  const canDelete = checkPermission(PERMISSIONS.MCP_CONFIGURATION_DELETE);

  return useMutation({
    mutationFn: (configurationId: string) => {
      if (!canDelete) {
        throw new Error("You do not have permission to delete MCP server configurations");
      }
      return mcpService.configurations.delete(accessToken!, configurationId);
    },
    onSuccess: () => {
      // Invalidate configurations list to refetch
      queryClient.invalidateQueries({
        queryKey: mcpQueryKeys.configurations.all,
      });
    },
  });
}

// Hook to sync MCP server tools
export function useSyncMCPServerTools() {
  const { accessToken } = useMetaInfo();
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (serverId: string) => {
      return mcpService.servers.syncTools(accessToken!, serverId);
    },
    onSuccess: (data, serverId) => {
      // Invalidate and refetch the specific server to get updated sync time and tools
      queryClient.invalidateQueries({
        queryKey: mcpQueryKeys.servers.detail(serverId),
      });
      // Also invalidate the servers list in case it affects the list view
      queryClient.invalidateQueries({
        queryKey: mcpQueryKeys.servers.all,
      });
    },
  });
}

// Hook to list operational MCP server configurations
export function useOperationalMCPServerConfigurations(params?: PaginationParams) {
  const { accessToken, activeOrg, activeRole } = useMetaInfo();

  // Create auth context key for cache separation without exposing token
  const authContextKey = activeOrg ? `${activeOrg.orgId}:${activeRole}` : undefined;

  return useQuery({
    queryKey: ["mcp", "configurations", "operational", params, authContextKey],
    queryFn: () => mcpService.configurations.listOperational(accessToken!, params),
    enabled: !!accessToken,
  });
}

// Hook to get a specific MCP tool by name
export function useMCPTool(toolName: string) {
  const { accessToken } = useMetaInfo();

  return useQuery({
    queryKey: mcpQueryKeys.tools.detail(toolName),
    queryFn: () => mcpService.tools.getByName(accessToken!, toolName),
    enabled: !!accessToken && !!toolName,
  });
}

// Hook to update an MCP server with permission check
export function useUpdateMCPServer() {
  const { accessToken, checkPermission } = useMetaInfo();
  const queryClient = useQueryClient();

  const canUpdate = checkPermission(PERMISSIONS.CUSTOM_MCP_SERVER_UPDATE);

  return useMutation({
    mutationFn: ({
      serverId,
      data,
    }: {
      serverId: string;
      data: { description?: string; logo?: string };
    }) => {
      if (!canUpdate) {
        throw new Error("You do not have permission to update MCP servers");
      }
      return mcpService.servers.update(accessToken!, serverId, data);
    },
    onSuccess: (updatedServer, { serverId }) => {
      // Update the specific server in the cache
      queryClient.setQueryData(mcpQueryKeys.servers.detail(serverId), updatedServer);
      // Also invalidate the servers list to refresh it
      queryClient.invalidateQueries({
        queryKey: mcpQueryKeys.servers.all,
      });
    },
  });
}

// Hook to delete an MCP server with permission check
export function useDeleteMCPServer() {
  const { accessToken, checkPermission } = useMetaInfo();
  const queryClient = useQueryClient();

  const canDelete = checkPermission(PERMISSIONS.CUSTOM_MCP_SERVER_DELETE);

  return useMutation({
    mutationFn: (serverId: string) => {
      if (!canDelete) {
        throw new Error("You do not have permission to delete MCP servers");
      }
      return mcpService.servers.delete(accessToken!, serverId);
    },
    onSuccess: () => {
      // Invalidate servers list to refetch
      queryClient.invalidateQueries({
        queryKey: mcpQueryKeys.servers.all,
      });
    },
  });
}
