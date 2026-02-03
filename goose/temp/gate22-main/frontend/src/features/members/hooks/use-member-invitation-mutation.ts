import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { useMetaInfo } from "@/components/context/metainfo";
import { inviteToOrganization } from "@/features/settings/api/organization";
import { QUERY_KEYS } from "@/features/settings/constants";
import { OrganizationInvitationDetail } from "@/features/invitations/types/invitation.types";

export interface MemberInvitationVariables {
  email: string;
  role: string;
}

interface UseMemberInvitationMutationOptions {
  invalidateMembers?: boolean;
  invalidateInvitations?: boolean;
  onSuccess?: (
    invitation: OrganizationInvitationDetail,
    variables: MemberInvitationVariables,
    context: unknown,
  ) => void | Promise<void>;
  onError?: (
    error: unknown,
    variables: MemberInvitationVariables | undefined,
    context: unknown,
  ) => void | Promise<void>;
}

export function useMemberInvitationMutation(options: UseMemberInvitationMutationOptions = {}) {
  const { accessToken, activeOrg } = useMetaInfo();
  const queryClient = useQueryClient();
  const invalidateMembers = options.invalidateMembers ?? false;
  const invalidateInvitations = options.invalidateInvitations ?? true;

  return useMutation<OrganizationInvitationDetail, unknown, MemberInvitationVariables>({
    mutationFn: async ({ email, role }) => {
      if (!accessToken || !activeOrg?.orgId) {
        throw new Error("Organization context unavailable");
      }

      return inviteToOrganization(accessToken, activeOrg.orgId, email, role);
    },
    onSuccess: async (invitation, variables, context) => {
      if (activeOrg?.orgId && invalidateMembers) {
        queryClient.invalidateQueries({
          queryKey: QUERY_KEYS.MEMBERS(activeOrg.orgId),
        });
      }

      if (activeOrg?.orgId && invalidateInvitations) {
        queryClient.invalidateQueries({
          queryKey: ["org-invitations", activeOrg.orgId],
        });
      }

      toast.success(`Invitation sent to ${variables.email}`);

      if (options.onSuccess) {
        await options.onSuccess(invitation, variables, context);
      }

      return invitation;
    },
    onError: async (error, variables, context) => {
      console.error("Failed to invite member:", error);
      toast.error("Failed to send invitation. Please try again.");

      if (options.onError) {
        await options.onError(error, variables, context);
      }
    },
  });
}
