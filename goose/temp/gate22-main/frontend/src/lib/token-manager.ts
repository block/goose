import { getApiBaseUrl } from "./api-client";
import { roleManager } from "./role-manager";
import { OrganizationRole } from "@/features/settings/types/organization.types";
import { CONTROL_PLANE_PATH } from "@/config/api.constants";

interface TokenResponse {
  token: string;
}

interface IssueTokenRequest {
  act_as?: {
    organization_id: string;
    role: string;
  };
}

class TokenManager {
  private static instance: TokenManager;
  private accessToken: string | null = null;
  private refreshPromise: Promise<string | null> | null = null;
  private currentActAs: IssueTokenRequest["act_as"] | undefined = undefined;

  private constructor() {}

  static getInstance(): TokenManager {
    if (!TokenManager.instance) {
      TokenManager.instance = new TokenManager();
    }
    return TokenManager.instance;
  }

  async getAccessToken(
    organizationId?: string,
    userActualRole?: OrganizationRole,
  ): Promise<string | null> {
    // Determine act_as payload. Default to the previously used act_as so
    // subsequent calls without explicit context keep the same organization.
    let nextActAs: IssueTokenRequest["act_as"] | undefined = this.currentActAs;

    if (organizationId) {
      let desiredRole: OrganizationRole | undefined;

      if (userActualRole === OrganizationRole.Admin) {
        const activeRole = roleManager.getActiveRole(organizationId);
        desiredRole = activeRole?.role ?? OrganizationRole.Admin;
      } else if (userActualRole) {
        desiredRole = userActualRole;
      } else if (this.currentActAs?.organization_id === organizationId && this.currentActAs.role) {
        desiredRole = this.currentActAs.role as OrganizationRole;
      }

      // Fallback to member to make sure act_as always carries the org context.
      if (!desiredRole) {
        desiredRole = OrganizationRole.Member;
      }

      nextActAs = {
        organization_id: organizationId,
        role: desiredRole,
      };
    }

    const actAsChanged =
      JSON.stringify(nextActAs ?? null) !== JSON.stringify(this.currentActAs ?? null);

    if (actAsChanged) {
      this.accessToken = null;

      // Wait for any existing refresh to complete before starting new one
      // to prevent older in-flight requests from overwriting newer tokens
      if (this.refreshPromise) {
        await this.refreshPromise;
      }

      this.refreshPromise = null;
    }

    this.currentActAs = nextActAs;

    // If we already have a valid token for the current act_as, reuse it.
    if (this.accessToken && !actAsChanged) {
      return this.accessToken;
    }

    // If a refresh is already running, share the promise.
    if (this.refreshPromise) {
      return this.refreshPromise;
    }

    // Otherwise, refresh the token with the resolved act_as context.
    return this.refreshAccessToken(nextActAs);
  }

  setAccessToken(token: string | null): void {
    this.accessToken = token;
  }

  getCurrentActAs(): IssueTokenRequest["act_as"] | undefined {
    return this.currentActAs;
  }

  clearToken(): void {
    this.accessToken = null;
    this.refreshPromise = null;
    this.currentActAs = undefined;
  }

  private async refreshAccessToken(act_as?: IssueTokenRequest["act_as"]): Promise<string | null> {
    this.refreshPromise = this.doRefreshToken(act_as);

    try {
      const token = await this.refreshPromise;
      return token;
    } finally {
      this.refreshPromise = null;
    }
  }

  private async doRefreshToken(act_as?: IssueTokenRequest["act_as"]): Promise<string | null> {
    const baseUrl = getApiBaseUrl();

    try {
      console.log("Refreshing access token");

      const response = await fetch(`${baseUrl}${CONTROL_PLANE_PATH}/auth/token`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        credentials: "include", // Include cookies for refresh token
        body: JSON.stringify({ act_as }),
      });

      if (!response.ok) {
        const errorText = await response.text();
        console.log(`Token refresh failed (${response.status}): ${errorText}`);

        // Clear token on auth failure
        if (response.status === 401) {
          this.clearToken();
          return null;
        }

        throw new Error(`Failed to refresh token: ${response.status}`);
      }

      const data: TokenResponse = await response.json();
      console.log("Token refresh successful");
      this.setAccessToken(data.token);
      return data.token;
    } catch (error) {
      console.error("Token refresh error:", error);
      // For network errors, re-throw
      if (error instanceof Error && !error.message.includes("401")) {
        throw error;
      }
      // For auth errors, return null
      this.clearToken();
      return null;
    }
  }
}

export const tokenManager = TokenManager.getInstance();
