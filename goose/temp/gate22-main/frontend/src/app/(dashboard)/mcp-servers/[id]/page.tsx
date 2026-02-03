"use client";

import { useParams, useRouter } from "next/navigation";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import {
  ArrowLeft,
  Wrench,
  Loader2,
  Plus,
  HelpCircle,
  RefreshCw,
  Settings,
  CheckCircle2,
  XCircle,
  Clock,
  Trash2,
  Edit,
} from "lucide-react";
import Image from "next/image";
import {
  useMCPServer,
  useSyncMCPServerTools,
  useOperationalMCPServerConfigurations,
  useDeleteMCPServer,
  useUpdateMCPServer,
} from "@/features/mcp/hooks/use-mcp-servers";
import { useState, useEffect } from "react";
import { MCPServerConfigurationStepper } from "@/features/mcp/components/mcp-server-configuration-stepper";
import { ToolsTable } from "@/features/mcp/components/tools-table";
import { OperationalAccountDialog } from "@/features/mcp/components/operational-account-dialog";
import { SyncResultsDialog } from "@/features/mcp/components/sync-results-dialog";
import { DeleteServerDialog } from "@/features/mcp/components/delete-server-dialog";
import { UpdateServerDialog } from "@/features/mcp/components/update-server-dialog";
import { PermissionGuard } from "@/components/rbac/permission-guard";
import { PERMISSIONS } from "@/lib/rbac/permissions";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { getAuthTypeLabel, getAuthTypeDetailedInfo } from "@/utils/auth-labels";
import { toast } from "sonner";
import { useMetaInfo } from "@/components/context/metainfo";
import { ToolsSyncResult } from "@/features/mcp/types/mcp.types";

export default function MCPServerDetailPage() {
  const params = useParams();
  const router = useRouter();
  const serverId = params.id as string;
  const [isConfigModalOpen, setIsConfigModalOpen] = useState(false);
  const [isOperationalDialogOpen, setIsOperationalDialogOpen] = useState(false);
  const [syncResults, setSyncResults] = useState<ToolsSyncResult | null>(null);
  const [isSyncResultsOpen, setIsSyncResultsOpen] = useState(false);
  const [isDeleteDialogOpen, setIsDeleteDialogOpen] = useState(false);
  const [isUpdateDialogOpen, setIsUpdateDialogOpen] = useState(false);
  const [, setForceUpdate] = useState(0);

  // Get organization context and permissions
  const { activeOrg } = useMetaInfo();

  // Fetch server data using the new hook
  const { data: server, isLoading, error } = useMCPServer(serverId);

  // Fetch operational configurations for this server
  const { data: operationalConfigs, refetch: refetchOperationalConfigs } =
    useOperationalMCPServerConfigurations();

  // Sync mutation
  const syncMutation = useSyncMCPServerTools();

  // Delete mutation
  const deleteMutation = useDeleteMCPServer();

  // Update mutation
  const updateMutation = useUpdateMCPServer();

  // Check if there's an operational account for this server
  const operationalConfig = operationalConfigs?.data?.find(
    (config) => config.mcp_server_id === serverId,
  );
  const hasOperationalAccount = operationalConfig?.has_operational_connected_account || false;

  // Check if enough time has passed since last sync
  // Current cooldown is 1 minute
  const canSyncByTime =
    !server?.last_synced_at ||
    new Date().getTime() - new Date(server.last_synced_at).getTime() >= 60 * 1000;

  // Update timer every minute when sync is on cooldown
  useEffect(() => {
    if (!server?.last_synced_at || canSyncByTime) return;

    const interval = setInterval(() => {
      // Force re-render to update the countdown
      setForceUpdate((prev) => prev + 1);
    }, 60000); // Update every minute

    return () => clearInterval(interval);
  }, [server?.last_synced_at, canSyncByTime]);

  const handleSync = () => {
    syncMutation.mutate(serverId, {
      onSuccess: (results) => {
        setSyncResults(results);
        setIsSyncResultsOpen(true);

        const totalChanges =
          results.tools_created.length +
          results.tools_deleted.length +
          results.tools_updated.length;

        if (totalChanges === 0) {
          toast.success("Tools are already up to date");
        } else {
          toast.success(`Tools synced successfully - ${totalChanges} changes made`);
        }
      },
      onError: (error) => {
        console.error("Sync failed:", error);
        toast.error("Failed to sync tools. Please try again.");
      },
    });
  };

  const handleUpdate = (data: { description?: string; logo?: string }) => {
    updateMutation.mutate(
      { serverId, data },
      {
        onSuccess: () => {
          toast.success(`MCP server "${server?.name}" updated successfully`);
          setIsUpdateDialogOpen(false);
        },
        onError: (error: Error) => {
          console.error("Update failed:", error);
          toast.error("Failed to update MCP server. Please try again.");
        },
      },
    );
  };

  const handleDelete = () => {
    deleteMutation.mutate(serverId, {
      onSuccess: () => {
        toast.success(`MCP server "${server?.name}" deleted successfully`);
        router.push("/mcp-servers");
      },
      onError: (error) => {
        console.error("Delete failed:", error);
        toast.error("Failed to delete MCP server. Please try again.");
      },
    });
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-12">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (error || !server) {
    return (
      <div className="p-6">
        <div className="py-12 text-center">
          <h2 className="mb-2 text-2xl font-semibold">Server Not Found</h2>
          <p className="mb-4 text-muted-foreground">The MCP server could not be found.</p>
          <Button variant="outline" onClick={() => router.push("/mcp-servers")}>
            <ArrowLeft className="mr-2 h-4 w-4" />
            Back to MCP Servers
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-7xl p-6">
      {/* Back Button */}
      <Button variant="outline" onClick={() => router.push("/mcp-servers")} className="mb-6">
        <ArrowLeft className="mr-2 h-4 w-4" />
        Back to MCP Servers
      </Button>

      {/* Header */}
      <div className="mb-6 flex items-center justify-between">
        <div className="flex items-center gap-4">
          <div className="relative h-12 w-12 shrink-0 overflow-hidden rounded-lg">
            <Image
              src={server.logo}
              alt={`${server.name} logo`}
              fill
              className="object-contain"
              unoptimized
            />
          </div>
          <div>
            <h1 className="text-2xl font-semibold">{server.name}</h1>
            {server.supported_auth_types && server.supported_auth_types.length > 0 && (
              <div className="mt-2 flex gap-1.5">
                {server.supported_auth_types.map((authType) => (
                  <div key={authType} className="flex items-center gap-1">
                    <Badge variant="secondary" className="text-xs">
                      {getAuthTypeLabel(authType)}
                    </Badge>
                    {getAuthTypeDetailedInfo(authType) && (
                      <Tooltip>
                        <TooltipTrigger asChild>
                          <button
                            type="button"
                            aria-label={`About ${getAuthTypeLabel(authType)}`}
                            className="m-0 inline-flex p-0"
                          >
                            <HelpCircle className="h-3.5 w-3.5 cursor-help text-muted-foreground transition-colors hover:text-foreground" />
                          </button>
                        </TooltipTrigger>
                        <TooltipContent className="max-w-xs">
                          <p>{getAuthTypeDetailedInfo(authType)}</p>
                        </TooltipContent>
                      </Tooltip>
                    )}
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
        <div className="flex items-center gap-2">
          <PermissionGuard permission={PERMISSIONS.MCP_CONFIGURATION_CREATE}>
            <Button onClick={() => setIsConfigModalOpen(true)}>
              <Plus className="mr-2 h-4 w-4" />
              Configure Server
            </Button>
          </PermissionGuard>
          {/* Update button - only show for custom servers (user's organization) and admin permission */}
          {server.organization_id === activeOrg?.orgId && (
            <PermissionGuard permission={PERMISSIONS.CUSTOM_MCP_SERVER_UPDATE}>
              <Button
                variant="outline"
                onClick={() => setIsUpdateDialogOpen(true)}
                disabled={updateMutation.isPending}
              >
                {updateMutation.isPending ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    Updating...
                  </>
                ) : (
                  <>
                    <Edit className="mr-2 h-4 w-4" />
                    Edit
                  </>
                )}
              </Button>
            </PermissionGuard>
          )}
          {/* Delete button - only show for custom servers (user's organization) and admin permission */}
          {server.organization_id === activeOrg?.orgId && (
            <PermissionGuard permission={PERMISSIONS.CUSTOM_MCP_SERVER_DELETE}>
              <Button
                variant="destructive"
                onClick={() => setIsDeleteDialogOpen(true)}
                disabled={deleteMutation.isPending}
              >
                {deleteMutation.isPending ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    Deleting...
                  </>
                ) : (
                  <>
                    <Trash2 className="mr-2 h-4 w-4" />
                    Delete
                  </>
                )}
              </Button>
            </PermissionGuard>
          )}
        </div>
      </div>

      {/* Description */}
      <p className="mb-6 text-muted-foreground">{server.description}</p>

      <Separator className="mb-4" />

      {/* Categories */}
      {server.categories.length > 0 && (
        <>
          <div className="mb-4">
            <h2 className="mb-3 text-lg font-semibold">Categories</h2>
            <div className="flex flex-wrap gap-2">
              {server.categories.map((category) => (
                <Badge key={category} variant="secondary" className="text-sm">
                  {category}
                </Badge>
              ))}
            </div>
          </div>
          <Separator className="mb-4" />
        </>
      )}

      {/* Operational Account - Only show for servers belonging to active organization */}
      {server.organization_id === activeOrg?.orgId && (
        <>
          <div className="mb-4">
            <div className="mb-3 flex justify-between">
              <div className="flex flex-col gap-4">
                <div className="flex items-center gap-2">
                  <Settings className="h-5 w-5" />
                  <h2 className="text-lg font-semibold">Operational Account</h2>
                </div>
                <div className="space-y-4">
                  <p className="text-sm text-muted-foreground">
                    The operational account is exclusively used by the system for administrative
                    purposes such as fetching MCP server metadata and monitoring server status. It
                    will never be used by any users.
                  </p>
                </div>
                <div className="flex gap-2">
                  {hasOperationalAccount ? (
                    <>
                      <CheckCircle2 className="h-4 w-4 text-green-500" />
                      <span className="text-sm text-muted-foreground">
                        Operational account is configured
                      </span>
                    </>
                  ) : (
                    <>
                      <XCircle className="h-4 w-4 text-red-500" />
                      <span className="text-sm text-muted-foreground">
                        No operational account configured
                      </span>
                    </>
                  )}
                </div>
              </div>
              <PermissionGuard permission={PERMISSIONS.CONNECTED_ACCOUNT_CREATE_OPERATIONAL}>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => setIsOperationalDialogOpen(true)}
                >
                  <Plus className="mr-2 h-4 w-4" />
                  {hasOperationalAccount ? "Update Account" : "Setup Account"}
                </Button>
              </PermissionGuard>
            </div>
          </div>

          <Separator className="mb-4" />
        </>
      )}

      {/* Tools */}
      <div className="mb-8">
        <div className="mb-4 flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Wrench className="h-5 w-5" />
            <h2 className="text-lg font-semibold">Available Tools ({server.tools?.length || 0})</h2>
          </div>
          {server.organization_id === activeOrg?.orgId && (
            <PermissionGuard permission={PERMISSIONS.CUSTOM_MCP_SERVER_SYNC}>
              <Tooltip>
                <TooltipTrigger asChild>
                  <div>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={handleSync}
                      disabled={syncMutation.isPending || !hasOperationalAccount || !canSyncByTime}
                    >
                      {syncMutation.isPending ? (
                        <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                      ) : !canSyncByTime ? (
                        <Clock className="mr-2 h-4 w-4" />
                      ) : (
                        <RefreshCw className="mr-2 h-4 w-4" />
                      )}
                      {server.last_synced_at ? "Re-sync now" : "Sync now"}
                    </Button>
                  </div>
                </TooltipTrigger>
                {(!hasOperationalAccount || !canSyncByTime) && (
                  <TooltipContent>
                    {!hasOperationalAccount ? (
                      <p>Setup an operational account first to enable syncing</p>
                    ) : !canSyncByTime ? (
                      <p>
                        Please wait a while before syncing again. You can re-sync the tools once
                        every minute.
                      </p>
                    ) : null}
                  </TooltipContent>
                )}
              </Tooltip>
            </PermissionGuard>
          )}
        </div>

        <p className="mb-4 text-sm text-muted-foreground">
          Last synced:{" "}
          {server.last_synced_at ? new Date(server.last_synced_at).toLocaleString() : "Never"}
        </p>

        <ToolsTable tools={server.tools || []} emptyMessage="No tools available for this server" />
      </div>

      {/* Configuration Stepper */}
      {server && (
        <MCPServerConfigurationStepper
          isOpen={isConfigModalOpen}
          onClose={() => setIsConfigModalOpen(false)}
          server={server}
        />
      )}

      {/* Operational Account Dialog - Only for servers belonging to active organization */}
      {server && server.organization_id === activeOrg?.orgId && (
        <OperationalAccountDialog
          open={isOperationalDialogOpen}
          onOpenChange={setIsOperationalDialogOpen}
          server={{
            id: server.id,
            name: server.name,
            auth_type: server.supported_auth_types?.[0], // Use first supported auth type
          }}
          operationalConfigId={operationalConfig?.id}
          onSuccess={() => {
            // Refresh operational configs to update the status and enable sync button
            refetchOperationalConfigs();
          }}
        />
      )}

      {/* Sync Results Dialog */}
      <SyncResultsDialog
        open={isSyncResultsOpen}
        onOpenChange={setIsSyncResultsOpen}
        results={syncResults}
        serverName={server?.name || ""}
      />

      {/* Update Server Dialog */}
      {server && (
        <UpdateServerDialog
          open={isUpdateDialogOpen}
          onOpenChange={setIsUpdateDialogOpen}
          server={server}
          onConfirm={handleUpdate}
          isPending={updateMutation.isPending}
        />
      )}

      {/* Delete Server Dialog */}
      {server && (
        <DeleteServerDialog
          open={isDeleteDialogOpen}
          onOpenChange={setIsDeleteDialogOpen}
          serverName={server.name}
          onConfirm={handleDelete}
          isPending={deleteMutation.isPending}
        />
      )}
    </div>
  );
}
