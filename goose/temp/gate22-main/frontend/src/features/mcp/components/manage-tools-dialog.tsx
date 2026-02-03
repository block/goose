"use client";

import { useState, useEffect } from "react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Switch } from "@/components/ui/switch";
import { Label } from "@/components/ui/label";
import { Loader2, Search } from "lucide-react";
import { Input } from "@/components/ui/input";
import { MCPServerConfigurationPublic } from "../types/mcp.types";
import { useMetaInfo } from "@/components/context/metainfo";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { mcpService } from "../api/mcp.service";
import { useMCPServer } from "../hooks/use-mcp-servers";
import { toast } from "sonner";

interface ManageToolsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  configuration: MCPServerConfigurationPublic;
  onUpdate: () => void;
}

export function ManageToolsDialog({
  open,
  onOpenChange,
  configuration,
  onUpdate,
}: ManageToolsDialogProps) {
  const { accessToken, activeOrg, activeRole } = useMetaInfo();
  const queryClient = useQueryClient();
  const [allToolsEnabled, setAllToolsEnabled] = useState(false);
  const [selectedTools, setSelectedTools] = useState<string[]>([]);
  const [searchQuery, setSearchQuery] = useState("");

  // Fetch the MCP server to get all available tools
  const { data: serverData } = useMCPServer(configuration.mcp_server_id);

  // Initialize state when dialog opens
  useEffect(() => {
    if (open) {
      setAllToolsEnabled(configuration.all_tools_enabled);
      if (!configuration.all_tools_enabled && configuration.enabled_tools) {
        setSelectedTools(configuration.enabled_tools.map((t) => t.id));
      } else {
        setSelectedTools([]);
      }
      setSearchQuery("");
    }
  }, [open, configuration]);

  // Create auth context key for cache invalidation
  const authContextKey = activeOrg ? `${activeOrg.orgId}:${activeRole}` : undefined;

  const updateMutation = useMutation({
    mutationFn: async () => {
      // Both all_tools_enabled and enabled_tools must be sent together
      return mcpService.configurations.update(accessToken, configuration.id, {
        all_tools_enabled: allToolsEnabled,
        enabled_tools: allToolsEnabled ? [] : selectedTools,
      });
    },
    onSuccess: () => {
      // Invalidate all relevant queries to refresh the data
      // Using the correct query keys that match the hooks
      queryClient.invalidateQueries({
        queryKey: ["mcp", "configurations", "detail", configuration.id, authContextKey],
      });
      queryClient.invalidateQueries({
        queryKey: ["mcp", "configurations", "list"],
      });
      queryClient.invalidateQueries({
        queryKey: ["mcp", "servers", "detail", configuration.mcp_server_id],
      });
      // Also invalidate the legacy query keys for backward compatibility
      queryClient.invalidateQueries({
        queryKey: ["mcp-server-configuration", configuration.id],
      });
      queryClient.invalidateQueries({
        queryKey: ["mcp-server-configurations"],
      });
      queryClient.invalidateQueries({
        queryKey: ["mcp-server", configuration.mcp_server_id],
      });
      toast.success("Tools updated successfully");
      onUpdate();
      onOpenChange(false);
    },
    onError: () => {
      toast.error("Failed to update tools");
    },
  });

  const handleToggleTool = (toolId: string) => {
    if (selectedTools.includes(toolId)) {
      setSelectedTools(selectedTools.filter((id) => id !== toolId));
    } else {
      setSelectedTools([...selectedTools, toolId]);
    }
  };

  const handleToggleAllTools = (checked: boolean) => {
    setAllToolsEnabled(checked);
    if (checked) {
      setSelectedTools([]);
    }
  };

  const handleSave = () => {
    updateMutation.mutate();
  };

  const filteredTools = serverData?.tools?.filter(
    (tool) =>
      tool.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      tool.description?.toLowerCase().includes(searchQuery.toLowerCase()),
  );

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[80vh] max-w-2xl">
        <DialogHeader>
          <DialogTitle>Manage Tools</DialogTitle>
          <DialogDescription>
            Configure which tools are available for this MCP configuration
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          {/* Enable All Tools Toggle - matching stepper style */}
          <div className="flex items-center justify-between rounded-lg border p-3">
            <div className="flex-1 space-y-0.5">
              <Label htmlFor="all-tools" className="text-sm font-medium">
                Enable all tools
              </Label>
              <p className="text-xs text-muted-foreground">
                Grant access to all {serverData?.tools?.length || 0} available tools
              </p>
            </div>
            <Switch
              id="all-tools"
              checked={allToolsEnabled}
              onCheckedChange={handleToggleAllTools}
            />
          </div>

          {/* Tool Selection (only shown when all tools is not enabled) */}
          {!allToolsEnabled && serverData?.tools && serverData.tools.length > 0 && (
            <div className="space-y-3">
              {/* Search Input with integrated icon */}
              <div className="relative">
                <Search className="absolute top-1/2 left-3 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
                <Input
                  placeholder="Search tools..."
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  className="pl-9"
                />
              </div>

              <div className="space-y-2">
                <Label className="text-xs font-medium text-muted-foreground">
                  Select specific tools:
                </Label>
                <div className="grid max-h-[300px] gap-1.5 overflow-y-auto rounded-lg border p-2 pr-2">
                  {filteredTools && filteredTools.length > 0 ? (
                    filteredTools.map((tool) => (
                      <label
                        key={tool.id}
                        className="flex cursor-pointer items-start space-x-2 rounded border p-2 transition-colors hover:bg-accent/50"
                      >
                        <Checkbox
                          id={tool.id}
                          checked={selectedTools.includes(tool.id)}
                          onCheckedChange={() => handleToggleTool(tool.id)}
                          className="mt-0.5"
                        />
                        <div className="min-w-0 flex-1">
                          <div className="text-sm font-medium">{tool.name}</div>
                          {tool.description && (
                            <p className="line-clamp-1 text-xs text-muted-foreground">
                              {tool.description}
                            </p>
                          )}
                        </div>
                      </label>
                    ))
                  ) : (
                    <p className="py-8 text-center text-sm text-muted-foreground">
                      {searchQuery ? "No tools match your search" : "No tools available"}
                    </p>
                  )}
                </div>
              </div>

              {selectedTools.length > 0 && (
                <p className="text-xs text-muted-foreground">
                  {selectedTools.length} tool
                  {selectedTools.length !== 1 ? "s" : ""} selected
                </p>
              )}
            </div>
          )}
        </div>

        <DialogFooter>
          <Button
            variant="outline"
            onClick={() => onOpenChange(false)}
            disabled={updateMutation.isPending}
          >
            Cancel
          </Button>
          <Button onClick={handleSave} disabled={updateMutation.isPending}>
            {updateMutation.isPending && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
            Save Changes
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
