import { useQuery } from "@tanstack/react-query";
import { useMetaInfo } from "@/components/context/metainfo";
import { subscriptionApi } from "../api/subscription";

export function usePlans() {
  const { activeOrg, accessToken } = useMetaInfo();

  return useQuery({
    queryKey: ["plans", activeOrg?.orgId],
    queryFn: () => subscriptionApi.getPlans(accessToken),
    enabled: !!accessToken && !!activeOrg?.orgId,
  });
}
