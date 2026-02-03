import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useMetaInfo } from "@/components/context/metainfo";
import { useRouter } from "next/navigation";
import { listOrganizationUsers, removeUser } from "@/features/settings/api/organization";
import { QUERY_KEYS } from "@/features/settings/constants";
import { toast } from "sonner";
import { OrganizationUser } from "@/features/settings/types/organization.types";
import { useMemberInvitationMutation } from "@/features/members/hooks/use-member-invitation-mutation";

export function useOrganizationMembers() {
  const { accessToken, activeOrg, user } = useMetaInfo();
  const queryClient = useQueryClient();
  const router = useRouter();

  const membersQuery = useQuery({
    queryKey: QUERY_KEYS.MEMBERS(activeOrg?.orgId || ""),
    queryFn: async () => {
      if (!accessToken || !activeOrg?.orgId) {
        throw new Error("Organization context unavailable");
      }

      return listOrganizationUsers(accessToken, activeOrg.orgId);
    },
    enabled: !!accessToken && !!activeOrg?.orgId,
    staleTime: 0,
    gcTime: 0,
    refetchOnMount: "always",
    refetchOnWindowFocus: "always",
  });

  const removeMemberMutation = useMutation({
    mutationFn: (userId: string) => removeUser(accessToken, activeOrg.orgId, userId),
    onSuccess: (_, userId) => {
      queryClient.invalidateQueries({
        queryKey: QUERY_KEYS.MEMBERS(activeOrg?.orgId || ""),
      });
      toast.success("Member removed successfully");

      // If the current user is leaving the organization
      if (userId === user.userId) {
        router.push("/mcp-servers");
      }
    },
    onError: (error) => {
      console.error("Failed to remove member:", error);
      toast.error("Failed to remove member");
    },
  });

  const memberInvitationMutation = useMemberInvitationMutation({
    invalidateMembers: true,
  });

  return {
    members: membersQuery.data as OrganizationUser[] | undefined,
    isLoading: membersQuery.isLoading,
    isError: membersQuery.isError,
    error: membersQuery.error,
    refetch: membersQuery.refetch,
    removeMember: removeMemberMutation.mutate,
    isRemoving: removeMemberMutation.isPending,
    createMemberInvitation: memberInvitationMutation.mutate,
    isInviting: memberInvitationMutation.isPending,
    createMemberInvitationAsync: memberInvitationMutation.mutateAsync,
  };
}
