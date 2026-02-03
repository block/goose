export interface Subscription {
  plan_code: string;
  seat_count: number;
  stripe_subscription_status:
    | "active"
    | "canceled"
    | "incomplete"
    | "incomplete_expired"
    | "past_due"
    | "trialing"
    | "unpaid";
  current_period_start: string;
  current_period_end: string;
  cancel_at_period_end: boolean;
}

export interface Entitlement {
  seat_count: number;
  max_custom_mcp_servers: number;
  log_retention_days: number;
}

export interface SubscriptionStatus {
  subscription: Subscription | null;
  entitlement: Entitlement;
  usage: OrganizationUsage;
  is_usage_exceeded: boolean;
}

export interface ChangeSubscriptionRequest {
  plan_code: string;
  seat_count?: number;
}

export interface ChangeSeatCountRequest {
  seat_count: number;
}

export interface ChangePlanRequest {
  plan_code: string;
  seat_count: number;
}

export interface ChangeSubscriptionResponse {
  url?: string;
}

export const PLAN_CODES = {
  FREE: "GATE22_FREE_PLAN",
  TEAM: "GATE22_TEAM_PLAN",
} as const;

export interface Plan {
  plan_code: string;
  display_name: string;
  max_seats_for_subscription?: number;
  max_custom_mcp_servers?: number;
  log_retention_days?: number;
}

export interface OrganizationUsage {
  seat_count: number;
  custom_mcp_servers_count: number;
}
