import { OrganizationUser } from "@/features/settings/types/organization.types";
import {
  OrganizationInvitationDetail,
  OrganizationInvitationStatus,
} from "@/features/invitations/types/invitation.types";
import { getApiBaseUrl } from "@/lib/api-client";
import { throwApiError } from "@/lib/api-error-handler";
import { CONTROL_PLANE_PATH } from "@/config/api.constants";

export async function listOrganizationUsers(
  accessToken: string,
  orgId: string,
): Promise<OrganizationUser[]> {
  const baseUrl = getApiBaseUrl();
  const response = await fetch(`${baseUrl}${CONTROL_PLANE_PATH}/organizations/${orgId}/members`, {
    headers: {
      Authorization: `Bearer ${accessToken}`,
    },
  });

  if (!response.ok) {
    await throwApiError(response, "Failed to fetch organization users");
  }

  const data = await response.json();

  // Transform backend data to match frontend expectations
  return data.map((member: OrganizationUser) => {
    // Parse name into first and last name
    const nameParts = member.name?.split(" ") || [];
    const firstName = nameParts[0] || "";
    const lastName = nameParts.slice(1).join(" ") || "";

    return {
      user_id: member.user_id,
      email: member.email,
      role: member.role,
      name: member.name,
      created_at: member.created_at,
      first_name: firstName,
      last_name: lastName,
    };
  });
}

export async function inviteToOrganization(
  accessToken: string,
  orgId: string,
  email: string,
  role: string,
): Promise<OrganizationInvitationDetail> {
  const baseUrl = getApiBaseUrl();
  const response = await fetch(
    `${baseUrl}${CONTROL_PLANE_PATH}/organizations/${orgId}/invitations`,
    {
      method: "POST",
      headers: {
        Authorization: `Bearer ${accessToken}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ email, role }),
    },
  );

  if (!response.ok) {
    await throwApiError(response, "Failed to invite user to organization");
  }

  return response.json();
}

export async function listOrganizationInvitations(
  accessToken: string,
  orgId: string,
  status?: OrganizationInvitationStatus,
): Promise<OrganizationInvitationDetail[]> {
  const baseUrl = getApiBaseUrl();
  const query = status ? `?status_filter=${encodeURIComponent(status)}` : "";
  const response = await fetch(
    `${baseUrl}${CONTROL_PLANE_PATH}/organizations/${orgId}/invitations${query}`,
    {
      headers: {
        Authorization: `Bearer ${accessToken}`,
      },
    },
  );

  if (!response.ok) {
    await throwApiError(response, "Failed to fetch organization invitations");
  }

  return response.json();
}

export async function cancelOrganizationInvitation(
  accessToken: string,
  orgId: string,
  invitationId: string,
): Promise<OrganizationInvitationDetail> {
  const baseUrl = getApiBaseUrl();
  const response = await fetch(
    `${baseUrl}${CONTROL_PLANE_PATH}/organizations/${orgId}/invitations/${invitationId}`,
    {
      method: "DELETE",
      headers: {
        Authorization: `Bearer ${accessToken}`,
      },
    },
  );

  if (!response.ok) {
    await throwApiError(response, "Failed to cancel organization invitation");
  }

  return response.json();
}

export async function removeUser(
  accessToken: string,
  orgId: string,
  userId: string,
): Promise<void> {
  const baseUrl = getApiBaseUrl();
  const response = await fetch(
    `${baseUrl}${CONTROL_PLANE_PATH}/organizations/${orgId}/members/${userId}`,
    {
      method: "DELETE",
      headers: {
        Authorization: `Bearer ${accessToken}`,
      },
    },
  );

  if (!response.ok) {
    await throwApiError(response, "Failed to remove user from organization");
  }
}

export async function createOrganization(
  accessToken: string,
  name: string,
  description?: string,
): Promise<{ organization_id: string; name: string; description?: string }> {
  const baseUrl = getApiBaseUrl();
  const response = await fetch(`${baseUrl}${CONTROL_PLANE_PATH}/organizations`, {
    method: "POST",
    headers: {
      Authorization: `Bearer ${accessToken}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ name, description }),
  });

  if (!response.ok) {
    await throwApiError(response, "Failed to create organization");
  }

  return response.json();
}
