"use client";

import { useMemo, useCallback, useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { Button } from "@/components/ui/button";
import { toast } from "sonner";
import Image from "next/image";
import { Settings, ArrowUpDown } from "lucide-react";
import { formatToLocalTime } from "@/utils/time";
import { EnhancedDataTable } from "@/components/ui-extensions/enhanced-data-table/data-table";
import { createColumnHelper, type ColumnDef } from "@tanstack/react-table";
import {
  useMCPServerConfigurations,
  useDeleteMCPServerConfiguration,
} from "@/features/mcp/hooks/use-mcp-servers";
import { MCPServerConfigurationPublicBasic } from "@/features/mcp/types/mcp.types";
import { PermissionGuard } from "@/components/rbac/permission-guard";
import { PERMISSIONS } from "@/lib/rbac/permissions";
import { usePermission } from "@/hooks/use-permissions";
import { Shield } from "lucide-react";
import { DeleteConfigurationDialog } from "@/features/mcp/components/delete-configuration-dialog";
import { Badge } from "@/components/ui/badge";
import { ConnectedAccountOwnership } from "@/features/mcp/types/mcp.types";
import { getOwnershipLabel } from "@/utils/configuration-labels";

const columnHelper = createColumnHelper<MCPServerConfigurationPublicBasic>();

export default function MCPConfigurationPage() {
  const router = useRouter();
  const canViewConfigurations = usePermission(PERMISSIONS.MCP_CONFIGURATION_PAGE_VIEW);
  const { data: configurationsResponse, isLoading } = useMCPServerConfigurations({ limit: 100 });
  const deleteConfiguration = useDeleteMCPServerConfiguration();
  const [deleteDialogState, setDeleteDialogState] = useState<{
    open: boolean;
    configurationId: string;
    configurationName: string;
  }>({ open: false, configurationId: "", configurationName: "" });

  // Redirect members who don't have permission
  useEffect(() => {
    if (!isLoading && !canViewConfigurations) {
      router.push("/mcp-servers");
    }
  }, [isLoading, canViewConfigurations, router]);

  const handleDelete = useCallback(async () => {
    try {
      await deleteConfiguration.mutateAsync(deleteDialogState.configurationId);
      toast.success(`Configuration "${deleteDialogState.configurationName}" deleted successfully`);
      setDeleteDialogState({
        open: false,
        configurationId: "",
        configurationName: "",
      });
    } catch (error) {
      console.error("Failed to delete configuration:", error);
      toast.error("Failed to delete configuration");
    }
  }, [deleteConfiguration, deleteDialogState.configurationId, deleteDialogState.configurationName]);

  const columns: ColumnDef<MCPServerConfigurationPublicBasic>[] = useMemo(() => {
    return [
      columnHelper.accessor("name", {
        id: "configuration_name",
        header: () => "CONFIGURATION NAME",
        cell: (info) => {
          const name = info.getValue();
          return <div className="font-medium">{name}</div>;
        },
        enableGlobalFilter: true,
      }),

      columnHelper.accessor((row) => row.mcp_server?.name, {
        id: "mcp_server_name",
        header: () => "MCP SERVER",
        cell: (info) => {
          const name = info.getValue();
          const logo = info.row.original.mcp_server?.logo;
          return (
            <div className="flex items-center gap-2">
              {logo && (
                <div className="relative h-5 w-5 shrink-0 overflow-hidden">
                  <Image
                    src={logo}
                    alt={`${name} logo`}
                    fill
                    className="rounded-sm object-contain"
                  />
                </div>
              )}
              <div className="font-medium">{name}</div>
            </div>
          );
        },
        enableGlobalFilter: true,
      }),

      columnHelper.accessor("connected_account_ownership", {
        id: "configuration_type",
        header: () => "TYPE",
        cell: (info) => {
          const type = info.getValue() as ConnectedAccountOwnership;
          if (!type) return null;

          return <Badge variant="outline">{getOwnershipLabel(type)}</Badge>;
        },
        enableGlobalFilter: true,
      }),

      columnHelper.accessor("created_at", {
        id: "created_at",
        header: ({ column }) => (
          <button
            onClick={() => column.toggleSorting(column.getIsSorted() === "asc")}
            className="flex items-center gap-1 transition-colors hover:text-foreground/80"
          >
            <span>CREATED</span>
            <ArrowUpDown className="h-4 w-4" />
          </button>
        ),
        cell: (info) => {
          const dateString = info.getValue();
          return <div className="text-sm">{dateString ? formatToLocalTime(dateString) : "-"}</div>;
        },
        enableGlobalFilter: false,
      }),

      columnHelper.accessor((row) => row, {
        id: "actions",
        header: () => <div className="flex justify-end">ACTIONS</div>,
        cell: (info) => {
          const configuration = info.getValue();
          return (
            <div className="flex items-center justify-end gap-2">
              <Button
                variant="outline"
                size="sm"
                onClick={() => router.push(`/mcp-configuration/${configuration.id}`)}
              >
                Detail
              </Button>

              <PermissionGuard permission={PERMISSIONS.MCP_CONFIGURATION_DELETE}>
                <Button
                  variant="destructive"
                  size="sm"
                  onClick={() =>
                    setDeleteDialogState({
                      open: true,
                      configurationId: configuration.id,
                      configurationName: configuration.name,
                    })
                  }
                >
                  Delete
                </Button>
              </PermissionGuard>
            </div>
          );
        },
        enableGlobalFilter: false,
      }),
    ] as ColumnDef<MCPServerConfigurationPublicBasic>[];
  }, [router]);

  // Show permission denied message for members
  if (!canViewConfigurations) {
    return (
      <div className="flex min-h-[60vh] flex-col items-center justify-center text-center">
        <Shield className="mb-4 h-12 w-12 text-muted-foreground" />
        <h3 className="mb-2 text-lg font-semibold">Access Restricted</h3>
        <p className="max-w-md text-muted-foreground">
          You don&apos;t have permission to manage MCP server configurations. Please contact your
          administrator for access.
        </p>
      </div>
    );
  }

  return (
    <div>
      <div className="border-b px-4 py-3">
        <h1 className="text-2xl font-bold">Configured MCP Servers</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          Manage your organization&apos;s MCP server configurations
        </p>
      </div>

      <div className="space-y-4 p-4">
        {isLoading ? (
          <div className="flex items-center justify-center p-8">
            <div className="flex flex-col items-center space-y-4">
              <div className="h-8 w-8 animate-spin rounded-full border-4 border-primary border-t-transparent"></div>
              <p className="text-sm text-muted-foreground">Loading configurations...</p>
            </div>
          </div>
        ) : !configurationsResponse || configurationsResponse.data.length === 0 ? (
          <div className="flex flex-col items-center justify-center p-16 text-center">
            <Settings className="mb-4 h-12 w-12 text-muted-foreground" />
            <h3 className="mb-2 text-lg font-semibold">No configurations yet</h3>
            <p className="mb-4 max-w-md text-muted-foreground">
              Get started by creating your first MCP server configuration. This will allow your
              organization to use MCP tools and integrations.
            </p>
          </div>
        ) : (
          <EnhancedDataTable
            columns={columns}
            data={configurationsResponse.data}
            defaultSorting={[{ id: "created_at", desc: true }]}
            searchBarProps={{
              placeholder: "Search configurations...",
            }}
            paginationOptions={{
              initialPageIndex: 0,
              initialPageSize: 15,
            }}
          />
        )}
      </div>

      {/* Delete Confirmation Dialog */}
      <DeleteConfigurationDialog
        open={deleteDialogState.open}
        onOpenChange={(open) => setDeleteDialogState((prev) => ({ ...prev, open }))}
        configurationName={deleteDialogState.configurationName}
        onConfirm={handleDelete}
        isPending={deleteConfiguration.isPending}
      />
    </div>
  );
}
