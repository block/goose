import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useRouter } from "next/navigation";
import { useMetaInfo } from "@/components/context/metainfo";
import { createTeam } from "@/features/teams/api/team";
import { CreateTeamRequest } from "@/features/teams/types/team.types";
import { QUERY_KEYS, UI_TEXT, SETTINGS_ROUTES } from "../constants";
import { toast } from "sonner";

export function useCreateTeam() {
  const router = useRouter();
  const queryClient = useQueryClient();
  const { activeOrg, accessToken } = useMetaInfo();

  const createTeamMutation = useMutation({
    mutationFn: (data: CreateTeamRequest) => {
      if (!data.name.trim()) {
        throw new Error(UI_TEXT.TEAM.NO_NAME_ERROR);
      }
      if (!activeOrg?.orgId) {
        throw new Error(UI_TEXT.TEAM.NO_ORG_ERROR);
      }
      return createTeam(accessToken, activeOrg.orgId, data);
    },
    onSuccess: (data, variables) => {
      queryClient.invalidateQueries({
        queryKey: QUERY_KEYS.TEAMS(activeOrg?.orgId || ""),
      });
      toast.success(UI_TEXT.TEAM.CREATE_SUCCESS(variables.name));
      router.push(SETTINGS_ROUTES.TEAMS);
    },
    onError: (error) => {
      console.error("Error creating team:", error);
      toast.error(error instanceof Error ? error.message : UI_TEXT.TEAM.CREATE_ERROR);
    },
  });

  return {
    createTeam: createTeamMutation.mutate,
    isCreating: createTeamMutation.isPending,
  };
}
