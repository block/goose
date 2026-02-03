import { getApiBaseUrl } from "@/lib/api-client";
import { parseApiError, throwApiError } from "@/lib/api-error-handler";
import { CONTROL_PLANE_PATH } from "@/config/api.constants";
import {
  OrganizationInvitationDetail,
  RespondInvitationPayload,
} from "@/features/invitations/types/invitation.types";
import { toast } from "sonner";

export async function getInvitationByToken(
  accessToken: string,
  organizationId: string,
  token: string,
): Promise<OrganizationInvitationDetail | null> {
  const baseUrl = getApiBaseUrl();
  const response = await fetch(
    `${baseUrl}${CONTROL_PLANE_PATH}/organizations/${organizationId}/invitations/get/${encodeURIComponent(
      token,
    )}`,
    {
      method: "GET",
      headers: {
        Authorization: `Bearer ${accessToken}`,
      },
    },
  );

  if (!response.ok) {
    const message = await parseApiError(response, "Failed to fetch invitation details");
    toast.error(message);
    return null;
  }

  return response.json();
}

export async function acceptInvitation(
  accessToken: string,
  organizationId: string,
  payload: RespondInvitationPayload,
): Promise<OrganizationInvitationDetail> {
  const baseUrl = getApiBaseUrl();
  const response = await fetch(
    `${baseUrl}${CONTROL_PLANE_PATH}/organizations/${organizationId}/invitations/accept/${encodeURIComponent(
      payload.token,
    )}`,
    {
      method: "POST",
      headers: {
        Authorization: `Bearer ${accessToken}`,
      },
    },
  );

  if (!response.ok) {
    await throwApiError(response, "Failed to accept invitation");
  }

  return response.json();
}

export async function rejectInvitation(
  accessToken: string,
  organizationId: string,
  payload: RespondInvitationPayload,
): Promise<OrganizationInvitationDetail> {
  const baseUrl = getApiBaseUrl();
  const response = await fetch(
    `${baseUrl}${CONTROL_PLANE_PATH}/organizations/${organizationId}/invitations/reject/${encodeURIComponent(
      payload.token,
    )}`,
    {
      method: "POST",
      headers: {
        Authorization: `Bearer ${accessToken}`,
      },
    },
  );

  if (!response.ok) {
    await throwApiError(response, "Failed to reject invitation");
  }

  return response.json();
}
