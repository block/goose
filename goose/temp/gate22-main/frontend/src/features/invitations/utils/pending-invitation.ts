import { PendingInvitationState } from "@/features/invitations/types/invitation.types";

const STORAGE_KEY = "pendingInvitation";

export function storePendingInvitation(state: PendingInvitationState): void {
  if (typeof window === "undefined") {
    return;
  }

  try {
    const payload: PendingInvitationState = {
      invitationId: state.invitationId ?? null,
      token: state.token,
      organizationId: state.organizationId ?? null,
    };

    sessionStorage.setItem(STORAGE_KEY, JSON.stringify(payload));
  } catch (error) {
    console.error("Failed to store pending invitation", error);
  }
}

export function getPendingInvitation(): PendingInvitationState | null {
  if (typeof window === "undefined") {
    return null;
  }

  try {
    const raw = sessionStorage.getItem(STORAGE_KEY);
    if (!raw) {
      return null;
    }

    const parsed = JSON.parse(raw) as PendingInvitationState;

    if (typeof parsed?.token !== "string" || !parsed.token.trim().length) {
      return null;
    }

    const invitationId =
      typeof parsed.invitationId === "string" && parsed.invitationId.trim().length
        ? parsed.invitationId
        : null;

    const organizationId =
      typeof parsed.organizationId === "string" && parsed.organizationId.trim().length
        ? parsed.organizationId
        : null;

    return {
      invitationId,
      token: parsed.token,
      organizationId,
    };
  } catch (error) {
    console.error("Failed to parse pending invitation", error);
    return null;
  }
}

export function clearPendingInvitation(): void {
  if (typeof window === "undefined") {
    return;
  }

  try {
    sessionStorage.removeItem(STORAGE_KEY);
  } catch (error) {
    console.error("Failed to clear pending invitation", error);
  }
}
