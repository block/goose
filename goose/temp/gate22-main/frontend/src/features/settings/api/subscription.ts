import { createAuthenticatedRequest } from "@/lib/api-client";
import {
  SubscriptionStatus,
  ChangeSubscriptionRequest,
  ChangeSeatCountRequest,
  ChangePlanRequest,
  ChangeSubscriptionResponse,
  Plan,
} from "../types/subscription.types";
import { CONTROL_PLANE_PATH } from "@/config/api.constants";

export const subscriptionApi = {
  getSubscriptionStatus: async (
    organizationId: string,
    token?: string,
  ): Promise<SubscriptionStatus> => {
    const api = createAuthenticatedRequest(token);
    return api.get<SubscriptionStatus>(
      `${CONTROL_PLANE_PATH}/subscriptions/organizations/${organizationId}/subscription-status`,
    );
  },

  changeSubscription: async (
    organizationId: string,
    data: ChangeSubscriptionRequest,
    token?: string,
  ): Promise<ChangeSubscriptionResponse> => {
    const api = createAuthenticatedRequest(token);
    return api.post<ChangeSubscriptionResponse>(
      `${CONTROL_PLANE_PATH}/subscriptions/organizations/${organizationId}/change-subscription`,
      data,
    );
  },

  changeSeatCount: async (
    organizationId: string,
    data: ChangeSeatCountRequest,
    token?: string,
  ): Promise<ChangeSubscriptionResponse> => {
    const api = createAuthenticatedRequest(token);
    return api.post<ChangeSubscriptionResponse>(
      `${CONTROL_PLANE_PATH}/subscriptions/organizations/${organizationId}/subscription-seat-change`,
      data,
    );
  },

  changePlan: async (
    organizationId: string,
    data: ChangePlanRequest,
    token?: string,
  ): Promise<ChangeSubscriptionResponse> => {
    const api = createAuthenticatedRequest(token);
    return api.post<ChangeSubscriptionResponse>(
      `${CONTROL_PLANE_PATH}/subscriptions/organizations/${organizationId}/subscription-plan-change`,
      data,
    );
  },

  getPlans: async (token?: string): Promise<Plan[]> => {
    const api = createAuthenticatedRequest(token);
    return api.get<Plan[]>(`${CONTROL_PLANE_PATH}/subscriptions/plans`);
  },

  cancelSubscription: async (organizationId: string, token?: string): Promise<void> => {
    const api = createAuthenticatedRequest(token);
    return api.post<void>(
      `${CONTROL_PLANE_PATH}/subscriptions/organizations/${organizationId}/cancel-subscription`,
    );
  },
};
