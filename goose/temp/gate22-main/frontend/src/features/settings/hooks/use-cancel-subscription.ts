import { useMutation } from "@tanstack/react-query";
import { useMetaInfo } from "@/components/context/metainfo";
import { subscriptionApi } from "../api/subscription";
import { toast } from "sonner";
import { useRouter } from "next/navigation";

export function useCancelSubscription() {
  const { activeOrg, accessToken } = useMetaInfo();
  const router = useRouter();

  const cancelSubscriptionMutation = useMutation({
    mutationFn: () => {
      if (!activeOrg?.orgId) {
        throw new Error("No organization selected");
      }
      return subscriptionApi.cancelSubscription(activeOrg.orgId, accessToken);
    },
    onSuccess: () => {
      // Redirect to cancellation success page
      // Note: Data invalidation is handled by the cancellation page itself
      router.push("/subscription/cancelled");
    },
    onError: (error) => {
      console.error("Error cancelling subscription:", error);
      toast.error(
        error instanceof Error ? error.message : "Failed to cancel subscription. Please try again.",
      );
    },
  });

  return {
    cancelSubscription: cancelSubscriptionMutation.mutate,
    isCancelling: cancelSubscriptionMutation.isPending,
  };
}
