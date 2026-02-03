import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { useMetaInfo } from "@/components/context/metainfo";
import {
  customMCPService,
  OAuth2DiscoveryRequest,
  OAuth2DCRRequest,
  CreateCustomMCPServerRequest,
} from "../api/custom-mcp.service";
import { mcpQueryKeys } from "./use-mcp-servers";

// Hook for OAuth2 discovery
export function useOAuth2Discovery() {
  const { accessToken, activeOrg, activeRole } = useMetaInfo();

  return useMutation({
    mutationFn: (request: OAuth2DiscoveryRequest) => {
      if (!accessToken) {
        throw new Error("Authentication required. Please log in.");
      }
      return customMCPService.discoverOAuth2(accessToken, activeOrg?.orgId, activeRole, request);
    },
    onSuccess: () => {
      toast.success("OAuth2 configuration discovered successfully");
    },
    onError: (error) => {
      console.error("Failed to discover OAuth2 configuration:", error);
      toast.error(
        error instanceof Error ? error.message : "Failed to discover OAuth2 configuration",
      );
    },
  });
}

// Hook for OAuth2 Dynamic Client Registration
export function useOAuth2ClientRegistration() {
  const { accessToken, activeOrg, activeRole } = useMetaInfo();

  return useMutation({
    mutationFn: (request: OAuth2DCRRequest) => {
      if (!accessToken) {
        throw new Error("Authentication required. Please log in.");
      }
      return customMCPService.registerOAuth2Client(
        accessToken,
        activeOrg?.orgId,
        activeRole,
        request,
      );
    },
    onSuccess: () => {
      toast.success("Client registered successfully");
    },
    onError: (error) => {
      console.error("Failed to register OAuth2 client:", error);
      toast.error(error instanceof Error ? error.message : "Failed to register OAuth2 client");
    },
  });
}

// Hook for creating a custom MCP server
export function useCreateCustomMCPServer() {
  const { accessToken, activeOrg, activeRole } = useMetaInfo();
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (request: CreateCustomMCPServerRequest) => {
      if (!accessToken) {
        throw new Error("Authentication required. Please log in.");
      }
      return customMCPService.create(accessToken, activeOrg?.orgId, activeRole, request);
    },
    onSuccess: (response) => {
      toast.success("Custom MCP server added successfully");
      // Invalidate relevant queries to refetch data
      queryClient.invalidateQueries({
        queryKey: mcpQueryKeys.servers.all,
      });
      queryClient.invalidateQueries({
        queryKey: mcpQueryKeys.configurations.all,
      });
      // Don't redirect immediately - let the component handle the next step
      return response;
    },
    onError: (error) => {
      console.error("Failed to create custom MCP server:", error);
      toast.error(error instanceof Error ? error.message : "Failed to create custom MCP server");
    },
  });
}
