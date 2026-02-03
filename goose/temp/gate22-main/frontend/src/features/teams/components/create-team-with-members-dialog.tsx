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
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { MultiSelect } from "@/components/ui/multi-select";
import { useMetaInfo } from "@/components/context/metainfo";
import { toast } from "sonner";
import { createTeam } from "@/features/teams/api/team";
import { listOrganizationUsers } from "@/features/settings/api/organization";
import { Team } from "@/features/teams/types/team.types";
import { OrganizationUser } from "@/features/settings/types/organization.types";
import { Loader2, Users } from "lucide-react";

interface CreateTeamWithMembersDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSuccess?: (team: Team) => void;
}

export function CreateTeamWithMembersDialog({
  open,
  onOpenChange,
  onSuccess,
}: CreateTeamWithMembersDialogProps) {
  const { accessToken, activeOrg } = useMetaInfo();
  const [isLoading, setIsLoading] = useState(false);
  const [formData, setFormData] = useState({
    name: "",
    description: "",
  });
  const [orgMembers, setOrgMembers] = useState<OrganizationUser[]>([]);
  const [selectedMembers, setSelectedMembers] = useState<string[]>([]);
  const [membersLoading, setMembersLoading] = useState(false);

  // Load organization members when dialog opens
  useEffect(() => {
    if (open) {
      loadOrganizationMembers();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  const loadOrganizationMembers = async () => {
    setMembersLoading(true);
    try {
      const members = await listOrganizationUsers(accessToken, activeOrg.orgId);
      setOrgMembers(members);
    } catch {
      toast.error("Failed to load organization members");
    } finally {
      setMembersLoading(false);
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!formData.name.trim()) {
      toast.error("Team name is required");
      return;
    }

    setIsLoading(true);
    try {
      const createTeamData = {
        ...formData,
        member_user_ids: selectedMembers.length > 0 ? selectedMembers : undefined,
      };

      const newTeam = await createTeam(accessToken, activeOrg.orgId, createTeamData);

      const successMessage =
        selectedMembers.length > 0
          ? `Team "${formData.name}" created with ${selectedMembers.length} member${selectedMembers.length > 1 ? "s" : ""}`
          : `Team "${formData.name}" created successfully`;
      toast.success(successMessage);

      // Reset form
      setFormData({
        name: "",
        description: "",
      });
      setSelectedMembers([]);

      onOpenChange(false);
      onSuccess?.(newTeam);
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : "Failed to create team";
      toast.error(errorMessage);
    } finally {
      setIsLoading(false);
    }
  };

  const handleClose = () => {
    if (!isLoading) {
      setFormData({
        name: "",
        description: "",
      });
      setSelectedMembers([]);
      onOpenChange(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={handleClose}>
      <DialogContent className="sm:max-w-[525px]">
        <form onSubmit={handleSubmit}>
          <DialogHeader>
            <DialogTitle>Create New Team</DialogTitle>
            <DialogDescription>
              Create a new team to organize members and manage access to resources.
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="name">
                Team Name <span className="text-destructive">*</span>
              </Label>
              <Input
                id="name"
                placeholder="Engineering, Marketing, Design..."
                value={formData.name}
                onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                disabled={isLoading}
                required
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="description">Description</Label>
              <Textarea
                id="description"
                placeholder="Describe the purpose of this team..."
                value={formData.description}
                onChange={(e) => setFormData({ ...formData, description: e.target.value })}
                disabled={isLoading}
                rows={3}
              />
            </div>

            <div className="space-y-2">
              <Label>Team Members (Optional)</Label>
              {membersLoading ? (
                <div className="flex items-center justify-center py-4">
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  <span className="text-sm text-muted-foreground">Loading members...</span>
                </div>
              ) : (
                <>
                  <MultiSelect
                    options={orgMembers.map((member) => ({
                      value: member.user_id,
                      label: `${member.name} (${member.email})`,
                    }))}
                    selected={selectedMembers}
                    onChange={setSelectedMembers}
                    placeholder="Select members to add..."
                    searchPlaceholder="Search members by name or email..."
                    emptyText="No members found."
                    className="w-full"
                    disabled={isLoading}
                  />
                  {selectedMembers.length > 0 && (
                    <div className="flex items-center gap-2 text-sm text-muted-foreground">
                      <Users className="h-4 w-4" />
                      <span>
                        {selectedMembers.length} member
                        {selectedMembers.length !== 1 ? "s" : ""} selected
                      </span>
                    </div>
                  )}
                </>
              )}
            </div>
          </div>

          <DialogFooter>
            <Button type="button" variant="outline" onClick={handleClose} disabled={isLoading}>
              Cancel
            </Button>
            <Button type="submit" disabled={isLoading}>
              {isLoading ? "Creating..." : "Create Team"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
