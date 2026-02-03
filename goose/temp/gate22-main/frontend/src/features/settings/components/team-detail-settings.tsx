"use client";

import { useState, useMemo } from "react";
import { useRouter } from "next/navigation";
import { useMetaInfo } from "@/components/context/metainfo";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { listTeamMembers, removeTeamMember, getTeam } from "@/features/teams/api/team";
import { AddTeamMemberDialog } from "@/features/teams/components/add-team-member-dialog";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Badge } from "@/components/ui/badge";
import { ArrowLeft, UserMinus, UserPlus, MoreHorizontal } from "lucide-react";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Skeleton } from "@/components/ui/skeleton";
import type { TeamMember } from "@/features/teams/types/team.types";

interface TeamDetailSettingsProps {
  teamId: string;
}

export function TeamDetailSettings({ teamId }: TeamDetailSettingsProps) {
  const router = useRouter();
  const queryClient = useQueryClient();
  const { accessToken, activeOrg } = useMetaInfo();
  const [removingMemberId, setRemovingMemberId] = useState<string | null>(null);
  const [showAddMemberDialog, setShowAddMemberDialog] = useState(false);

  const { data: team, isLoading: teamLoading } = useQuery({
    queryKey: ["team", activeOrg?.orgId, teamId],
    queryFn: () => getTeam(accessToken, activeOrg!.orgId, teamId),
    enabled: !!accessToken && !!activeOrg?.orgId && !!teamId,
  });

  const { data: members, isLoading: membersLoading } = useQuery({
    queryKey: ["team-members", activeOrg?.orgId, teamId],
    queryFn: () => listTeamMembers(accessToken, activeOrg!.orgId, teamId),
    enabled: !!accessToken && !!activeOrg?.orgId && !!teamId,
  });

  const removeMemberMutation = useMutation({
    mutationFn: (userId: string) => removeTeamMember(accessToken, activeOrg!.orgId, teamId, userId),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: ["team-members", activeOrg?.orgId, teamId],
      });
      toast.success("Member removed successfully");
      setRemovingMemberId(null);
    },
    onError: (error) => {
      console.error("Failed to remove member:", error);
      toast.error("Failed to remove member");
      setRemovingMemberId(null);
    },
  });

  const getInitials = useMemo(
    () => (name: string) => {
      return name
        .split(" ")
        .map((word) => word[0])
        .join("")
        .toUpperCase()
        .slice(0, 2);
    },
    [],
  );

  const handleRemoveMember = (userId: string) => {
    setRemovingMemberId(userId);
    removeMemberMutation.mutate(userId);
  };

  const renderLoadingState = () => (
    <div>
      <div className="mb-8">
        <Skeleton className="mb-4 h-8 w-32" />
        <Skeleton className="mb-2 h-10 w-64" />
        <Skeleton className="h-6 w-96" />
      </div>
      <div className="space-y-4">
        <div>
          <Skeleton className="mb-2 h-7 w-48" />
          <Skeleton className="h-5 w-32" />
        </div>
        <div className="space-y-4">
          {[1, 2, 3].map((i) => (
            <div key={i} className="flex items-center justify-between rounded-lg border p-4">
              <div className="flex items-center gap-3">
                <Skeleton className="h-10 w-10 rounded-full" />
                <div>
                  <Skeleton className="mb-1 h-5 w-32" />
                  <Skeleton className="h-4 w-40" />
                </div>
              </div>
              <div className="flex items-center gap-2">
                <Skeleton className="h-6 w-16 rounded-full" />
                <Skeleton className="h-8 w-8 rounded" />
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );

  if (teamLoading || membersLoading) {
    return renderLoadingState();
  }

  const renderMemberCard = (member: TeamMember) => (
    <div key={member.user_id} className="flex items-center justify-between rounded-lg border p-4">
      <div className="flex items-center gap-3">
        <Avatar className="h-10 w-10">
          <AvatarFallback className="bg-gray-100 font-medium text-gray-900 dark:bg-gray-800 dark:text-gray-100">
            {getInitials(member.name)}
          </AvatarFallback>
        </Avatar>
        <div>
          <p className="font-medium">{member.name}</p>
          <p className="text-sm text-muted-foreground">{member.email}</p>
        </div>
      </div>
      <div className="flex items-center gap-2">
        <Badge variant="secondary">{member.role}</Badge>
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="ghost" size="icon" className="h-8 w-8">
              <span className="sr-only">Open menu</span>
              <MoreHorizontal className="h-4 w-4" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuLabel>Actions</DropdownMenuLabel>
            <DropdownMenuSeparator />
            <DropdownMenuItem
              className="text-destructive"
              onClick={() => handleRemoveMember(member.user_id)}
              disabled={removingMemberId === member.user_id}
            >
              <UserMinus className="mr-2 h-4 w-4" />
              Remove from team
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </div>
  );

  return (
    <div>
      <div className="mb-8">
        <Button variant="ghost" size="sm" className="mb-4" onClick={() => router.push("/teams")}>
          <ArrowLeft className="mr-2 h-4 w-4" />
          Back to Teams
        </Button>

        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-3xl font-bold tracking-tight">{team?.name}</h1>
            {team?.description && <p className="mt-2 text-muted-foreground">{team.description}</p>}
          </div>
          <div className="flex gap-2">
            <Button onClick={() => setShowAddMemberDialog(true)}>
              <UserPlus className="mr-2 h-4 w-4" />
              Add Members
            </Button>
          </div>
        </div>
      </div>

      <div className="space-y-4">
        <div>
          <h2 className="text-lg font-semibold">Team Members</h2>
          <p className="text-sm text-muted-foreground">
            {members?.length || 0} {members?.length === 1 ? "member" : "members"} in this team
          </p>
        </div>

        {members && members.length > 0 ? (
          <div className="space-y-4">{members.map(renderMemberCard)}</div>
        ) : (
          <div className="py-12 text-center text-muted-foreground">
            <UserPlus className="mx-auto mb-3 h-12 w-12 opacity-50" />
            <p className="text-sm">No members in this team yet</p>
            <p className="mt-1 text-xs">Click &ldquo;Add Members&rdquo; to add team members</p>
          </div>
        )}
      </div>

      <AddTeamMemberDialog
        teamId={teamId}
        open={showAddMemberDialog}
        onOpenChange={setShowAddMemberDialog}
        onSuccess={() => {
          queryClient.invalidateQueries({
            queryKey: ["team-members", activeOrg?.orgId, teamId],
          });
        }}
      />
    </div>
  );
}
