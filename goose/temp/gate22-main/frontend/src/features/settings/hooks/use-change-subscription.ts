import { useMutation } from "@tanstack/react-query";
import { useMetaInfo } from "@/components/context/metainfo";
import { subscriptionApi } from "../api/subscription";
import {
  ChangeSubscriptionRequest,
  ChangeSeatCountRequest,
  ChangePlanRequest,
} from "../types/subscription.types";
import { toast } from "sonner";
import { useRouter } from "next/navigation";

export function useChangeSubscription() {
  const { activeOrg, accessToken } = useMetaInfo();
  const router = useRouter();

  const changeSubscriptionMutation = useMutation({
    mutationFn: (data: ChangeSubscriptionRequest) => {
      if (!activeOrg?.orgId) {
        throw new Error("No organization selected");
      }
      return subscriptionApi.changeSubscription(activeOrg.orgId, data, accessToken);
    },
    onSuccess: (response) => {
      if (response.url) {
        // Redirect to Stripe checkout
        window.location.href = response.url;
      } else {
        // Redirect to subscription updated page
        // Note: Data invalidation is handled by the updated page itself
        router.push("/subscription/updated");
      }
    },
    onError: (error) => {
      console.error("Error changing subscription:", error);
      toast.error(
        error instanceof Error ? error.message : "Failed to change subscription. Please try again.",
      );
    },
  });

  return {
    changeSubscription: changeSubscriptionMutation.mutate,
    isChanging: changeSubscriptionMutation.isPending,
  };
}

export function useChangeSeatCount() {
  const { activeOrg, accessToken } = useMetaInfo();
  const router = useRouter();

  const changeSeatCountMutation = useMutation({
    mutationFn: (data: ChangeSeatCountRequest) => {
      if (!activeOrg?.orgId) {
        throw new Error("No organization selected");
      }
      return subscriptionApi.changeSeatCount(activeOrg.orgId, data, accessToken);
    },
    onSuccess: (response) => {
      if (response.url) {
        // Redirect to Stripe checkout
        window.location.href = response.url;
      } else {
        // Redirect to subscription updated page
        // Note: Data invalidation is handled by the updated page itself
        router.push("/subscription/updated");
      }
    },
    onError: (error) => {
      console.error("Error changing seat count:", error);
      toast.error(
        error instanceof Error ? error.message : "Failed to change seat count. Please try again.",
      );
    },
  });

  return {
    changeSeatCount: changeSeatCountMutation.mutate,
    isChanging: changeSeatCountMutation.isPending,
  };
}

export function useChangePlan() {
  const { activeOrg, accessToken } = useMetaInfo();
  const router = useRouter();

  const changePlanMutation = useMutation({
    mutationFn: (data: ChangePlanRequest) => {
      if (!activeOrg?.orgId) {
        throw new Error("No organization selected");
      }
      return subscriptionApi.changePlan(activeOrg.orgId, data, accessToken);
    },
    onSuccess: (response) => {
      if (response.url) {
        // Redirect to Stripe checkout
        window.location.href = response.url;
      } else {
        // Redirect to subscription updated page
        // Note: Data invalidation is handled by the updated page itself
        router.push("/subscription/updated");
      }
    },
    onError: (error) => {
      console.error("Error changing plan:", error);
      toast.error(
        error instanceof Error ? error.message : "Failed to change plan. Please try again.",
      );
    },
  });

  return {
    changePlan: changePlanMutation.mutate,
    isChanging: changePlanMutation.isPending,
  };
}
