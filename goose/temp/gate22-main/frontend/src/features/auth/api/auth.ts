import { getApiBaseUrl } from "@/lib/api-client";
import { toast } from "sonner";
import { CONTROL_PLANE_PATH } from "@/config/api.constants";
import { storePendingInvitation } from "@/features/invitations/utils/pending-invitation";
import { sanitizeRedirectPath } from "@/lib/safe-redirect";

// Request/Response types
export interface EmailLoginRequest {
  email: string;
  password: string;
}

export interface EmailRegistrationRequest {
  name: string;
  email: string;
  password: string;
  redirect_path?: string;
}

export interface TokenResponse {
  token: string;
}

export interface UserInfo {
  user_id: string;
  name: string;
  email: string;
  organizations: Array<{
    organization_id: string;
    organization_name: string;
    role: string;
  }>;
}

export interface IssueTokenRequest {
  act_as?: {
    organization_id: string;
    role: string;
  };
}

// Auth functions
export async function register(data: EmailRegistrationRequest): Promise<boolean> {
  const baseUrl = getApiBaseUrl();
  const response = await fetch(`${baseUrl}${CONTROL_PLANE_PATH}/auth/register/email`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    credentials: "include", // Include cookies
    body: JSON.stringify(data),
  });

  if (!response.ok) {
    try {
      const errorData = await response.json();
      // Handle different error response formats
      const errorMessage = errorData.detail || errorData.message || errorData.error;

      // Provide user-friendly messages for common errors
      if (typeof errorMessage === "string") {
        const normalizedError = errorMessage.toLowerCase();

        // Map backend error codes to user-friendly messages
        if (
          normalizedError === "email_already_exists" ||
          normalizedError === "user_already_exists" ||
          normalizedError.includes("email already")
        ) {
          toast.error("This email is already registered. Please try logging in instead.");
          return false;
        }
        toast.error(errorMessage);
        return false;
      }
      toast.error("Registration failed. Please try again.");
      return false;
    } catch {
      toast.error("Registration failed. Please try again.");
      return false;
    }
  }

  toast.success("Registration successful!");
  return true;
}

export interface LoginResult {
  success: boolean;
  redirectTo?: string | null;
}

export async function login(email: string, password: string): Promise<LoginResult> {
  const baseUrl = getApiBaseUrl();
  const response = await fetch(`${baseUrl}${CONTROL_PLANE_PATH}/auth/login/email`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    credentials: "include", // Include cookies
    body: JSON.stringify({
      email,
      password,
    } as EmailLoginRequest),
  });

  if (!response.ok) {
    try {
      const errorData = await response.json();
      // Handle different error response formats
      const errorMessage = errorData.detail || errorData.message || errorData.error;

      // Provide user-friendly messages for common errors
      if (typeof errorMessage === "string") {
        const normalizedError = errorMessage.toLowerCase();

        if (normalizedError.includes("invalid") || normalizedError.includes("incorrect")) {
          toast.error("Invalid email or password. Please try again.");
          return { success: false };
        }
        if (
          normalizedError === "email_not_verified" ||
          normalizedError.includes("email not verified")
        ) {
          toast.error(
            "Please verify your email before logging in. Check your inbox or request a new verification email.",
          );
          return { success: false };
        }
        if (normalizedError === "user_not_found" || normalizedError.includes("not found")) {
          toast.error("No account found with this email. Please sign up first.");
          return { success: false };
        }
        if (
          normalizedError === "account_deletion_in_progress" ||
          normalizedError === "account deletion in progress"
        ) {
          toast.error(
            "Your account is currently being deleted. Please try again later or contact support.",
          );
          return { success: false };
        }
        toast.error(errorMessage);
        return { success: false };
      }
      toast.error("Login failed. Please try again.");
      return { success: false };
    } catch {
      toast.error("Login failed. Please try again.");
      return { success: false };
    }
  }

  let redirectTo: string | null = null;

  try {
    const payload = await response.json();
    const rawRedirect = payload?.redirect_to;

    if (typeof rawRedirect === "string" && rawRedirect.trim().length > 0) {
      const origin = typeof window !== "undefined" ? window.location.origin : "http://localhost";
      const redirectUrl = new URL(rawRedirect, origin);
      const invitationId = redirectUrl.searchParams.get("invitation_id");
      const organizationId = redirectUrl.searchParams.get("organization_id");
      const token = redirectUrl.searchParams.get("token");

      if (invitationId && token) {
        storePendingInvitation({
          invitationId,
          token,
          organizationId: organizationId ?? null,
        });
      }

      redirectTo = sanitizeRedirectPath(rawRedirect);
    }
  } catch (error) {
    if (!(error instanceof SyntaxError)) {
      console.error("Failed to parse login response", error);
    }
  }

  toast.success("Login successful!");
  return { success: true, redirectTo };
}

export async function issueToken(act_as?: IssueTokenRequest["act_as"]): Promise<TokenResponse> {
  const baseUrl = getApiBaseUrl();
  const response = await fetch(`${baseUrl}${CONTROL_PLANE_PATH}/auth/token`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    credentials: "include", // Include cookies
    body: JSON.stringify({ act_as }),
  });

  if (!response.ok) {
    try {
      const errorData = await response.json();
      const errorMessage = errorData.detail || errorData.message || errorData.error;

      if (typeof errorMessage === "string") {
        throw new Error(errorMessage);
      }
      throw new Error("Failed to issue token. Please try again.");
    } catch (e) {
      if (e instanceof Error) {
        throw e;
      }
      throw new Error("Failed to issue token. Please try again.");
    }
  }

  return response.json();
}

export async function logout(): Promise<void> {
  const baseUrl = getApiBaseUrl();
  await fetch(`${baseUrl}${CONTROL_PLANE_PATH}/auth/logout`, {
    method: "POST",
    credentials: "include", // Include cookies
  });
}

export async function getProfile(token: string): Promise<UserInfo> {
  const baseUrl = getApiBaseUrl();
  const response = await fetch(`${baseUrl}${CONTROL_PLANE_PATH}/users/me/profile`, {
    method: "GET",
    headers: {
      Authorization: `Bearer ${token}`,
    },
  });

  if (!response.ok) {
    try {
      const errorData = await response.json();
      const errorMessage = errorData.detail || errorData.message || errorData.error;

      if (typeof errorMessage === "string") {
        throw new Error(errorMessage);
      }
      throw new Error("Failed to fetch user profile. Please try again.");
    } catch (e) {
      if (e instanceof Error) {
        throw e;
      }
      throw new Error("Failed to fetch user profile. Please try again.");
    }
  }

  return response.json();
}

// Google OAuth helper function
export function getGoogleAuthUrl(nextPath?: string): string {
  const baseUrl = getApiBaseUrl();
  const callbackUrl = new URL(`${window.location.origin}/callback`);
  callbackUrl.searchParams.set("provider", "google");

  if (nextPath && nextPath.startsWith("/")) {
    callbackUrl.searchParams.set("next", nextPath);
  }

  const redirectUri = encodeURIComponent(callbackUrl.toString());
  return `${baseUrl}${CONTROL_PLANE_PATH}/auth/google/authorize?redirect_uri=${redirectUri}`;
}
