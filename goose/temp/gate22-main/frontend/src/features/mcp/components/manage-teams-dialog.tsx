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
import { MultiSelect } from "@/components/ui/multi-select";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Loader2, AlertCircle, Plus } from "lucide-react";
import { MCPServerConfigurationPublic } from "../types/mcp.types";
import { useMetaInfo } from "@/components/context/metainfo";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { mcpService } from "../api/mcp.service";
import { listTeams } from "@/features/teams/api/team";
import { toast } from "sonner";
import { CreateTeamDialog } from "@/features/settings/components/create-team-dialog";

interface ManageTeamsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  configuration: MCPServerConfigurationPublic;
  onUpdate: () => void;
}

export function ManageTeamsDialog({
  open,
  onOpenChange,
  configuration,
  onUpdate,
}: ManageTeamsDialogProps) {
  const { accessToken, activeOrg, activeRole } = useMetaInfo();
  const queryClient = useQueryClient();
  const [selectedTeams, setSelectedTeams] = useState<string[]>([]);
  const [showCreateTeam, setShowCreateTeam] = useState(false);

  // Fetch all teams in the organization
  const { data: allTeams, isLoading } = useQuery({
    queryKey: ["teams", activeOrg?.orgId],
    queryFn: () => listTeams(accessToken, activeOrg!.orgId),
    enabled: !!accessToken && !!activeOrg?.orgId && open,
  });

  // Initialize selected teams when dialog opens
  useEffect(() => {
    if (open && configuration.allowed_teams) {
      setSelectedTeams(configuration.allowed_teams.map((t) => t.team_id));
    }
  }, [open, configuration.allowed_teams]);

  // Create auth context key for cache invalidation
  const authContextKey = activeOrg ? `${activeOrg.orgId}:${activeRole}` : undefined;

  const updateMutation = useMutation({
    mutationFn: async () => {
      return mcpService.configurations.update(accessToken, configuration.id, {
        allowed_teams: selectedTeams,
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
      // Also invalidate the legacy query keys for backward compatibility
      queryClient.invalidateQueries({
        queryKey: ["mcp-server-configuration", configuration.id],
      });
      queryClient.invalidateQueries({
        queryKey: ["mcp-server-configurations"],
      });
      toast.success("Teams updated successfully");
      onUpdate();
      onOpenChange(false);
    },
    onError: () => {
      toast.error("Failed to update teams");
    },
  });

  const handleSave = () => {
    updateMutation.mutate();
  };

  const handleCancel = () => {
    // Reset selected teams to original state
    if (configuration.allowed_teams) {
      setSelectedTeams(configuration.allowed_teams.map((t) => t.team_id));
    } else {
      setSelectedTeams([]);
    }
    onOpenChange(false);
  };

  return (
    <>
      <Dialog open={open} onOpenChange={onOpenChange}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>Manage Teams</DialogTitle>
            <DialogDescription>
              Select which teams can access this MCP configuration
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4">
            {isLoading ? (
              <div className="flex items-center justify-center py-8">
                <Loader2 className="mr-2 h-5 w-5 animate-spin" />
                <span className="text-sm text-muted-foreground">Loading teams...</span>
              </div>
            ) : !allTeams || allTeams.length === 0 ? (
              <div className="space-y-4">
                <Alert>
                  <AlertCircle className="h-4 w-4" />
                  <AlertDescription className="text-xs">
                    No teams found. Create a team to manage access to this MCP configuration.
                  </AlertDescription>
                </Alert>
                <Button
                  onClick={() => setShowCreateTeam(true)}
                  className="w-full"
                  variant="outline"
                >
                  <Plus className="mr-2 h-4 w-4" />
                  Create New Team
                </Button>
              </div>
            ) : (
              <div className="space-y-3">
                <div className="flex gap-2">
                  <MultiSelect
                    options={allTeams.map((team) => ({
                      value: team.team_id,
                      label: `${team.name}${team.member_count !== undefined ? ` (${team.member_count} member${team.member_count !== 1 ? "s" : ""})` : ""}`,
                    }))}
                    selected={selectedTeams}
                    onChange={setSelectedTeams}
                    placeholder="Select teams..."
                    searchPlaceholder="Search teams..."
                    emptyText="No teams found."
                    className="flex-1"
                  />
                  <Button
                    onClick={() => setShowCreateTeam(true)}
                    variant="outline"
                    size="icon"
                    title="Create New Team"
                    className="min-h-[40px] w-[40px] shrink-0"
                  >
                    <Plus className="h-4 w-4" />
                  </Button>
                </div>
                {selectedTeams.length > 0 && (
                  <p className="text-xs text-muted-foreground">
                    {selectedTeams.length} team
                    {selectedTeams.length !== 1 ? "s" : ""} selected
                  </p>
                )}
              </div>
            )}
          </div>

          <DialogFooter className="flex items-center justify-between">
            <div className="text-sm text-muted-foreground">
              {selectedTeams.length} {selectedTeams.length === 1 ? "team" : "teams"} selected
            </div>
            <div className="flex gap-2">
              <Button variant="outline" onClick={handleCancel} disabled={updateMutation.isPending}>
                Cancel
              </Button>
              <Button onClick={handleSave} disabled={updateMutation.isPending}>
                {updateMutation.isPending && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
                Save Changes
              </Button>
            </div>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <CreateTeamDialog
        open={showCreateTeam}
        onOpenChange={setShowCreateTeam}
        onSuccess={() => {
          // Refetch teams after successful creation
          queryClient.invalidateQueries({
            queryKey: ["teams", activeOrg?.orgId],
          });
          setShowCreateTeam(false);
        }}
      />
    </>
  );
}
