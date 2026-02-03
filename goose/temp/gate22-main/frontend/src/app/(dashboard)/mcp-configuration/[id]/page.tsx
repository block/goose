"use client";

import { useState } from "react";
import { useParams, useRouter } from "next/navigation";
import {
  useMCPServerConfiguration,
  useMCPServer,
  useDeleteMCPServerConfiguration,
} from "@/features/mcp/hooks/use-mcp-servers";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { ArrowLeft, Loader2, AlertCircle, Edit2, Settings, HelpCircle } from "lucide-react";
import Image from "next/image";
import { ToolsTable } from "@/features/mcp/components/tools-table";
import { useRole } from "@/hooks/use-permissions";
import { useMetaInfo } from "@/components/context/metainfo";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { mcpService } from "@/features/mcp/api/mcp.service";
import { toast } from "sonner";
import { ManageTeamsDialog } from "@/features/mcp/components/manage-teams-dialog";
import { ManageToolsDialog } from "@/features/mcp/components/manage-tools-dialog";
import { DeleteConfigurationDialog } from "@/features/mcp/components/delete-configuration-dialog";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { getAuthTypeLabel, getAuthTypeDetailedInfo } from "@/utils/auth-labels";
import { getConfigurationTypeDetailedInfo, getOwnershipLabel } from "@/utils/configuration-labels";
import { ConnectedAccountOwnership } from "@/features/mcp/types/mcp.types";

export default function MCPConfigurationDetailPage() {
  const params = useParams();
  const router = useRouter();
  const queryClient = useQueryClient();
  const configurationId = params.id as string;
  const { isAdmin, isActingAsMember } = useRole();
  const shouldShowAdminLink = isAdmin && !isActingAsMember;
  const { accessToken, activeOrg, activeRole } = useMetaInfo();

  const [isEditingName, setIsEditingName] = useState(false);
  const [editedName, setEditedName] = useState("");
  const [editedDescription, setEditedDescription] = useState("");
  const [showManageTeams, setShowManageTeams] = useState(false);
  const [showManageTools, setShowManageTools] = useState(false);
  const [isDeleteDialogOpen, setIsDeleteDialogOpen] = useState(false);

  const { data: configuration, isLoading, error } = useMCPServerConfiguration(configurationId);

  // Fetch full server data when all_tools_enabled is true to get the tools list
  const { data: serverData } = useMCPServer(
    configuration?.all_tools_enabled ? configuration.mcp_server_id : "",
  );

  // Create auth context key for cache invalidation
  const authContextKey = activeOrg ? `${activeOrg.orgId}:${activeRole}` : undefined;

  const deleteConfiguration = useDeleteMCPServerConfiguration();

  const updateConfigMutation = useMutation({
    mutationFn: async (data: { name?: string; description?: string }) => {
      return mcpService.configurations.update(accessToken, configurationId, data);
    },
    onSuccess: () => {
      // Invalidate with the correct query keys that match the hooks
      queryClient.invalidateQueries({
        queryKey: ["mcp", "configurations", "detail", configurationId, authContextKey],
      });
      queryClient.invalidateQueries({
        queryKey: ["mcp", "configurations", "list"],
      });
      // Also invalidate the legacy query keys for backward compatibility
      queryClient.invalidateQueries({
        queryKey: ["mcp-server-configuration", configurationId],
      });
      queryClient.invalidateQueries({
        queryKey: ["mcp-server-configurations"],
      });
      toast.success("Configuration updated successfully");
      setIsEditingName(false);
    },
    onError: () => {
      toast.error("Failed to update configuration");
    },
  });

  const handleSaveNameDescription = () => {
    const updates: { name?: string; description?: string } = {};
    if (editedName && editedName !== configuration?.name) {
      updates.name = editedName;
    }
    if (editedDescription !== configuration?.description) {
      updates.description = editedDescription || "";
    }
    if (Object.keys(updates).length > 0) {
      updateConfigMutation.mutate(updates);
    } else {
      setIsEditingName(false);
    }
  };

  const startEditingName = () => {
    if (configuration) {
      setEditedName(configuration.name);
      setEditedDescription(configuration.description || "");
      setIsEditingName(true);
    }
  };

  const handleDelete = async () => {
    try {
      await deleteConfiguration.mutateAsync(configurationId);
      toast.success("Configuration deleted successfully");
      router.push("/mcp-configuration");
    } catch (error) {
      console.error("Failed to delete configuration:", error);
      toast.error("Failed to delete configuration");
    }
  };

  if (isLoading) {
    return (
      <div className="flex min-h-[400px] items-center justify-center">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (error || !configuration) {
    return (
      <div className="container mx-auto max-w-5xl p-6">
        <Button
          variant="outline"
          onClick={() => router.push("/mcp-configuration")}
          className="mb-4"
        >
          <ArrowLeft className="mr-2 h-4 w-4" />
          Back to Configurations
        </Button>
        <Card>
          <CardContent className="pt-6">
            <div className="flex items-center gap-2 text-destructive">
              <AlertCircle className="h-5 w-5" />
              <p>Failed to load configuration details</p>
            </div>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="container mx-auto max-w-7xl space-y-6 p-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <Button variant="outline" onClick={() => router.push("/mcp-configuration")}>
          <ArrowLeft className="mr-2 h-4 w-4" />
          Back to Configurations
        </Button>
      </div>

      {/* Configuration Overview */}
      <Card>
        <CardHeader>
          <div className="flex items-start justify-between">
            <div className="flex items-start gap-4">
              <div className="flex flex-col items-center gap-2">
                {configuration.mcp_server.logo && (
                  <Image
                    src={configuration.mcp_server.logo}
                    alt={configuration.mcp_server.name}
                    width={48}
                    height={48}
                    className="rounded"
                  />
                )}
                {shouldShowAdminLink ? (
                  <Button
                    variant="link"
                    className="h-auto p-0 text-xs"
                    onClick={() => router.push(`/mcp-servers/${configuration.mcp_server_id}`)}
                  >
                    {configuration.mcp_server.name}
                  </Button>
                ) : (
                  <span className="text-center text-xs">{configuration.mcp_server.name}</span>
                )}
              </div>
              <div className="flex-1">
                {isEditingName ? (
                  <div className="space-y-3">
                    <div className="flex items-center gap-2">
                      <Input
                        value={editedName}
                        onChange={(e) => setEditedName(e.target.value)}
                        className="h-auto py-1 text-2xl font-semibold"
                        placeholder="Configuration name"
                      />
                      <Button
                        size="sm"
                        onClick={handleSaveNameDescription}
                        disabled={updateConfigMutation.isPending}
                      >
                        Save
                      </Button>
                      <Button
                        size="sm"
                        variant="outline"
                        onClick={() => setIsEditingName(false)}
                        disabled={updateConfigMutation.isPending}
                      >
                        Cancel
                      </Button>
                    </div>
                    <Textarea
                      value={editedDescription}
                      onChange={(e) => setEditedDescription(e.target.value)}
                      className="resize-none text-sm"
                      placeholder="Configuration description (optional)"
                      rows={2}
                    />
                  </div>
                ) : (
                  <div className="group">
                    <div className="flex items-center gap-2">
                      <CardTitle className="text-2xl">{configuration.name}</CardTitle>
                      {isAdmin && !isActingAsMember && (
                        <Button
                          size="icon"
                          variant="ghost"
                          className="opacity-0 transition-opacity group-hover:opacity-100"
                          onClick={startEditingName}
                        >
                          <Edit2 className="h-4 w-4" />
                        </Button>
                      )}
                    </div>
                    {configuration.description && (
                      <p className="mt-2 text-sm text-muted-foreground">
                        {configuration.description}
                      </p>
                    )}
                  </div>
                )}
              </div>
            </div>
            {/* Delete Configuration Button - Top Right */}
            {isAdmin && !isActingAsMember && (
              <Button variant="destructive" size="sm" onClick={() => setIsDeleteDialogOpen(true)}>
                Delete
              </Button>
            )}
          </div>
        </CardHeader>
        <CardContent className="border-t pt-6">
          <div className="grid grid-cols-1 gap-6 md:grid-cols-4">
            <div>
              <label className="text-sm font-medium text-muted-foreground">
                Connected Account Type
              </label>
              <div className="mt-1 flex items-center gap-2">
                <p className="text-sm font-medium">
                  {getOwnershipLabel(configuration.connected_account_ownership)}
                </p>
                {configuration.connected_account_ownership && (
                  <Tooltip>
                    <TooltipTrigger asChild>
                      <HelpCircle
                        className="h-3.5 w-3.5 cursor-help text-muted-foreground transition-colors hover:text-foreground focus:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
                        aria-label="Connected account type information"
                        tabIndex={0}
                      />
                    </TooltipTrigger>
                    <TooltipContent className="max-w-xs">
                      <p>
                        {getConfigurationTypeDetailedInfo(
                          configuration.connected_account_ownership as ConnectedAccountOwnership,
                        )}
                      </p>
                    </TooltipContent>
                  </Tooltip>
                )}
              </div>
            </div>

            <div>
              <label className="text-sm font-medium text-muted-foreground">
                Authentication Type
              </label>
              <div className="mt-1 flex items-center gap-2">
                <p className="text-sm font-medium">{getAuthTypeLabel(configuration.auth_type)}</p>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <HelpCircle
                      className="h-3.5 w-3.5 cursor-help text-muted-foreground transition-colors hover:text-foreground focus:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
                      aria-label="Authentication type information"
                      tabIndex={0}
                    />
                  </TooltipTrigger>
                  <TooltipContent className="max-w-xs">
                    <p>{getAuthTypeDetailedInfo(configuration.auth_type)}</p>
                  </TooltipContent>
                </Tooltip>
              </div>
            </div>

            {configuration.created_at && (
              <div>
                <label className="text-sm font-medium text-muted-foreground">Created At</label>
                <p className="mt-1 text-sm">
                  {new Date(configuration.created_at).toLocaleString()}
                </p>
              </div>
            )}

            {configuration.updated_at && (
              <div>
                <label className="text-sm font-medium text-muted-foreground">Updated At</label>
                <p className="mt-1 text-sm">
                  {new Date(configuration.updated_at).toLocaleString()}
                </p>
              </div>
            )}
          </div>
        </CardContent>
      </Card>

      {/* Enabled Teams */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <CardTitle>Enabled Teams</CardTitle>
          {isAdmin && !isActingAsMember && (
            <Button size="sm" variant="outline" onClick={() => setShowManageTeams(true)}>
              <Settings className="mr-2 h-4 w-4" />
              Manage Teams
            </Button>
          )}
        </CardHeader>
        <CardContent>
          {configuration.allowed_teams && configuration.allowed_teams.length > 0 ? (
            <div className="rounded-lg border">
              <table className="w-full">
                <thead>
                  <tr className="border-b bg-muted/50">
                    <th className="p-3 text-left text-sm font-medium">Team Name</th>
                    <th className="w-20 p-3 text-center text-sm font-medium">Action</th>
                  </tr>
                </thead>
                <tbody>
                  {configuration.allowed_teams.map((team) => (
                    <tr key={team.team_id} className="border-b last:border-0">
                      <td className="p-3">
                        <code className="text-sm">{team.name}</code>
                      </td>
                      <td className="p-3 text-center">
                        <Button
                          size="sm"
                          variant="outline"
                          className="h-7 px-2 text-xs"
                          onClick={() => router.push(`/teams/${team.team_id}`)}
                        >
                          View
                        </Button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : (
            <p className="text-sm text-muted-foreground">
              No teams are assigned to this configuration.
            </p>
          )}
        </CardContent>
      </Card>

      {/* Enabled Tools */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <div className="flex items-center">
            <CardTitle>
              Enabled Tools
              {configuration.all_tools_enabled && (
                <Badge className="ml-2" variant="secondary">
                  All Tools Enabled
                </Badge>
              )}
            </CardTitle>
          </div>
          {isAdmin && !isActingAsMember && (
            <Button size="sm" variant="outline" onClick={() => setShowManageTools(true)}>
              <Settings className="mr-2 h-4 w-4" />
              Manage Tools
            </Button>
          )}
        </CardHeader>
        <CardContent>
          {configuration.all_tools_enabled ? (
            serverData?.tools && serverData.tools.length > 0 ? (
              <ToolsTable
                tools={serverData.tools}
                emptyMessage="This configuration has access to all available tools from the MCP server."
              />
            ) : (
              <p className="text-sm text-muted-foreground">
                This configuration has access to all available tools from the MCP server.
              </p>
            )
          ) : configuration.enabled_tools && configuration.enabled_tools.length > 0 ? (
            <ToolsTable
              tools={configuration.enabled_tools}
              emptyMessage="No specific tools are enabled for this configuration."
            />
          ) : (
            <p className="text-sm text-muted-foreground">
              No specific tools are enabled for this configuration.
            </p>
          )}
        </CardContent>
      </Card>

      {/* Dialogs */}
      {configuration && (
        <>
          <ManageTeamsDialog
            open={showManageTeams}
            onOpenChange={setShowManageTeams}
            configuration={configuration}
            onUpdate={() => {
              queryClient.invalidateQueries({
                queryKey: ["mcp-server-configuration", configurationId],
              });
            }}
          />
          <ManageToolsDialog
            open={showManageTools}
            onOpenChange={setShowManageTools}
            configuration={configuration}
            onUpdate={() => {
              queryClient.invalidateQueries({
                queryKey: ["mcp-server-configuration", configurationId],
              });
            }}
          />
          <DeleteConfigurationDialog
            open={isDeleteDialogOpen}
            onOpenChange={setIsDeleteDialogOpen}
            configurationName={configuration.name}
            onConfirm={handleDelete}
            isPending={deleteConfiguration.isPending}
          />
        </>
      )}
    </div>
  );
}
