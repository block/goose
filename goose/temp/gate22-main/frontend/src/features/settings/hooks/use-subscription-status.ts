import { useQuery } from "@tanstack/react-query";
import { subscriptionApi } from "../api/subscription";
import { SubscriptionStatus } from "../types/subscription.types";
import { useMetaInfo } from "@/components/context/metainfo";
import { QUERY_KEYS } from "../constants";

export function useSubscriptionStatus() {
  const { activeOrg, accessToken } = useMetaInfo();

  return useQuery<SubscriptionStatus>({
    queryKey: QUERY_KEYS.SUBSCRIPTION_STATUS(activeOrg?.orgId || ""),
    queryFn: () => {
      if (!accessToken || !activeOrg?.orgId) {
        throw new Error("Missing access token or organization ID");
      }
      return subscriptionApi.getSubscriptionStatus(activeOrg.orgId, accessToken);
    },
    enabled: !!accessToken && !!activeOrg?.orgId,
  });
}
