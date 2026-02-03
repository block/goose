import { useQuery } from "@tanstack/react-query";
import { useMetaInfo } from "@/components/context/metainfo";
import { listTeams } from "../api/team";
import { Team } from "../types/team.types";

export function useTeams() {
  const { activeOrg, accessToken } = useMetaInfo();

  return useQuery<Team[]>({
    queryKey: ["teams", activeOrg?.orgId],
    queryFn: () => {
      if (!accessToken || !activeOrg?.orgId) {
        throw new Error("Missing access token or organization ID");
      }
      return listTeams(accessToken, activeOrg.orgId);
    },
    enabled: !!accessToken && !!activeOrg?.orgId,
  });
}
