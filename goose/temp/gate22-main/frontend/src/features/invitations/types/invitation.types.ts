export enum OrganizationInvitationStatus {
  Pending = "pending",
  Accepted = "accepted",
  Rejected = "rejected",
  Canceled = "canceled",
}

export interface OrganizationInvitationDetail {
  invitation_id: string;
  organization_id: string;
  email: string;
  inviter_user_id: string;
  inviter_name: string | null;
  role: string;
  status: OrganizationInvitationStatus;
  expires_at: string;
  used_at: string | null;
  created_at: string;
  updated_at: string;
  email_metadata?: Record<string, unknown> | null;
}

export interface RespondInvitationPayload {
  token: string;
}

export interface PendingInvitationState {
  token: string;
  invitationId?: string | null;
  organizationId?: string | null;
}
