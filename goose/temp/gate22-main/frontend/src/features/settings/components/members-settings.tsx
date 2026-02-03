"use client";

import { useState } from "react";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { Button } from "@/components/ui/button";
import { UserPlus } from "lucide-react";
import { VercelMembersTable } from "@/features/members/components/vercel-members-table";
import { MemberInvitationDialog } from "@/features/members/components/member-invitation-dialog";
import { PendingInvitationsTable } from "@/features/members/components/pending-invitations-table";
import { useOrganizationMemberInvitations } from "@/features/members/hooks/use-organization-member-invitations";
import { OrganizationInvitationStatus } from "@/features/invitations/types/invitation.types";

export function MembersSettings() {
  const [inviteDialogOpen, setInviteDialogOpen] = useState(false);
  const [refreshKey, setRefreshKey] = useState(0);
  const [activeTab, setActiveTab] = useState("team-members");

  const {
    invitations: pendingInvitations,
    isLoading: isLoadingInvitations,
    refetch: refetchInvitations,
    cancelInvitationAsync,
    isCancelling,
    createMemberInvitationAsync,
    isInviting,
  } = useOrganizationMemberInvitations({
    status: OrganizationInvitationStatus.Pending,
  });

  const handleInviteSuccess = () => {
    setInviteDialogOpen(false);
    setActiveTab("pending");
    setRefreshKey((prev) => prev + 1);
    void refetchInvitations();
  };

  return (
    <div className="space-y-6">
      <div className="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
        <div>
          <h2 className="text-2xl font-bold tracking-tight">Members</h2>
          <p className="text-muted-foreground">Manage organization members and invitations</p>
        </div>
        <Button onClick={() => setInviteDialogOpen(true)} className="w-full md:w-auto">
          <UserPlus className="mr-2 h-4 w-4" />
          Invite Member
        </Button>
      </div>

      <Tabs value={activeTab} onValueChange={setActiveTab} className="w-full">
        <TabsList className="grid h-full w-full grid-cols-1 gap-2 sm:w-full sm:max-w-xl sm:grid-cols-2 md:w-fit">
          <TabsTrigger
            value="team-members"
            className="h-10 min-w-[180px] justify-center whitespace-nowrap"
          >
            Active Members
          </TabsTrigger>
          <TabsTrigger
            value="pending"
            className="h-10 min-w-[180px] justify-center whitespace-nowrap"
          >
            <span className="flex items-center gap-2">
              <span>Pending Invitations</span>
              {pendingInvitations?.length ? (
                <span className="rounded-full bg-muted px-2 py-0.5 text-xs">
                  {pendingInvitations.length}
                </span>
              ) : null}
            </span>
          </TabsTrigger>
        </TabsList>

        <TabsContent value="team-members" className="mt-4">
          <VercelMembersTable refreshKey={refreshKey} />
        </TabsContent>

        <TabsContent value="pending" className="mt-4">
          <PendingInvitationsTable
            invitations={pendingInvitations}
            isLoading={isLoadingInvitations}
            onCancel={cancelInvitationAsync}
            isCancelling={isCancelling}
          />
        </TabsContent>
      </Tabs>

      <MemberInvitationDialog
        open={inviteDialogOpen}
        onOpenChange={setInviteDialogOpen}
        onSuccess={handleInviteSuccess}
        onInvite={({ email, role }) => createMemberInvitationAsync({ email, role })}
        isInviting={isInviting}
      />
    </div>
  );
}
