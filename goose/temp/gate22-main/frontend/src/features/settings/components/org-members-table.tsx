import { useEffect, useState, useMemo } from "react";
import { EnhancedDataTable } from "@/components/ui-extensions/enhanced-data-table/data-table";
import { useOrgMembersTableColumns } from "@/features/members/hooks/use-org-members-table-columns";
import { OrganizationUser, OrganizationRole } from "@/features/settings/types/organization.types";
import {
  listOrganizationUsers,
  inviteToOrganization,
  removeUser,
} from "@/features/settings/api/organization";
import { useMetaInfo } from "@/components/context/metainfo";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { toast } from "sonner";
import {
  Dialog,
  DialogTrigger,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
  DialogClose,
  DialogDescription,
} from "@/components/ui/dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Plus } from "lucide-react";
import { useRouter } from "next/navigation";

export function OrgMembersTable() {
  const { accessToken, activeOrg, user } = useMetaInfo();
  const router = useRouter();
  const [members, setMembers] = useState<OrganizationUser[]>([]);
  const [inviteEmail, setInviteEmail] = useState("");
  const [inviteRole, setInviteRole] = useState<OrganizationRole>(OrganizationRole.Admin);
  const [inviting, setInviting] = useState(false);
  const [open, setOpen] = useState(false);

  // Determine available roles for inviting
  // const currentRole = activeOrg.userAssignedRole as OrganizationRole;
  // const roleHierarchy = [OrganizationRole.Owner, OrganizationRole.Admin];
  // const currentRoleIndex = roleHierarchy.indexOf(currentRole);
  // const availableRoles = roleHierarchy.slice(currentRoleIndex);

  const fetchMembers = useMemo(
    () => async () => {
      try {
        const data = await listOrganizationUsers(accessToken, activeOrg.orgId);
        setMembers(data);
      } catch {
        toast.error("Failed to load organization members");
      }
    },
    [accessToken, activeOrg.orgId],
  );

  useEffect(() => {
    fetchMembers();
  }, [fetchMembers]);

  const handleInvite = async () => {
    if (!inviteEmail) return;
    setInviting(true);
    try {
      await inviteToOrganization(accessToken, activeOrg.orgId, inviteEmail, inviteRole);
      toast.success("Invitation sent");
      setInviteEmail("");
      setOpen(false);
      fetchMembers();
    } catch {
      toast.error("Failed to invite user");
    } finally {
      setInviting(false);
    }
  };

  const handleRemove = async (userId: string) => {
    try {
      await removeUser(accessToken, activeOrg.orgId, userId);
      toast.success("Member removed");
      fetchMembers();

      // If the current user is leaving the organization
      if (userId === user.userId) {
        // Navigate to the MCP servers page
        router.push("/mcp-servers");
      }
    } catch {
      toast.error("Failed to remove member");
    }
  };

  const columns = useOrgMembersTableColumns({
    onRemove: handleRemove,
  });

  return (
    <div className="space-y-4">
      <div className="-mt-6 mb-2 flex items-center gap-2">
        <Dialog open={open} onOpenChange={setOpen}>
          <DialogTrigger asChild className="relative top-14 ml-auto">
            <Button onClick={() => setOpen(true)}>
              <Plus className="h-4 w-4" />
              Invite user
            </Button>
          </DialogTrigger>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Invite User</DialogTitle>
              <DialogDescription>
                Enter the email and select a role to invite a new member to your organization.
              </DialogDescription>
            </DialogHeader>
            <div className="flex flex-col gap-4 py-2">
              <Input
                placeholder="Enter email address"
                value={inviteEmail}
                onChange={(e) => setInviteEmail(e.target.value)}
                className="w-full"
                autoFocus
              />
              <Select
                value={inviteRole}
                onValueChange={(v) => setInviteRole(v as OrganizationRole)}
              >
                <SelectTrigger>
                  <SelectValue placeholder="Select role" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value={OrganizationRole.Admin}>Admin</SelectItem>
                  <SelectItem value={OrganizationRole.Member}>Member</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <DialogFooter>
              <DialogClose asChild>
                <Button variant="outline">Cancel</Button>
              </DialogClose>
              <Button onClick={handleInvite} disabled={inviting || !inviteEmail || !inviteRole}>
                {inviting ? "Inviting..." : "Invite"}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </div>
      <EnhancedDataTable
        columns={columns}
        data={members}
        searchBarProps={{ placeholder: "Search by email" }}
        paginationOptions={{ initialPageIndex: 0, initialPageSize: 10 }}
      />
    </div>
  );
}
