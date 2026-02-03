import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useMetaInfo } from "@/components/context/metainfo";
import { listTeamMembers, removeTeamMember } from "@/features/teams/api/team";
import { QUERY_KEYS, UI_TEXT } from "../constants";
import { toast } from "sonner";

export function useTeamMembers(teamId: string) {
  const { accessToken, activeOrg } = useMetaInfo();
  const queryClient = useQueryClient();

  const membersQuery = useQuery({
    queryKey: QUERY_KEYS.TEAM_MEMBERS(activeOrg?.orgId || "", teamId),
    queryFn: () => listTeamMembers(accessToken, activeOrg.orgId, teamId),
    enabled: !!accessToken && !!activeOrg?.orgId && !!teamId,
  });

  const removeMemberMutation = useMutation({
    mutationFn: (userId: string) => removeTeamMember(accessToken, activeOrg.orgId, teamId, userId),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: QUERY_KEYS.TEAM_MEMBERS(activeOrg?.orgId || "", teamId),
      });
      toast.success(UI_TEXT.TEAM.REMOVE_MEMBER_SUCCESS);
    },
    onError: (error) => {
      console.error("Failed to remove member:", error);
      toast.error(UI_TEXT.TEAM.REMOVE_MEMBER_ERROR);
    },
  });

  return {
    members: membersQuery.data,
    isLoading: membersQuery.isLoading,
    removeMember: removeMemberMutation.mutate,
    isRemoving: removeMemberMutation.isPending,
  };
}
