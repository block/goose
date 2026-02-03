"use client";

import { createContext, ReactNode, useContext, useState, useEffect, useCallback } from "react";
import { useRouter } from "next/navigation";
import { useQueryClient } from "@tanstack/react-query";
import { Skeleton } from "@/components/ui/skeleton";
import { getProfile, logout as apiLogout } from "@/features/auth/api/auth";
import { tokenManager } from "@/lib/token-manager";
import { roleManager } from "@/lib/role-manager";
import { organizationManager } from "@/lib/organization-manager";
import { OrganizationRole } from "@/features/settings/types/organization.types";
import { checkPermission, getPermissionsForRole } from "@/lib/rbac/rbac-service";
import { Permission } from "@/lib/rbac/permissions";
import { mcpQueryKeys } from "@/features/mcp/hooks/use-mcp-servers";
import { connectedAccountKeys } from "@/features/connected-accounts/hooks/use-connected-account";

export interface UserClass {
  userId: string;
  email: string;
  firstName?: string;
  lastName?: string;
  username?: string;
  pictureUrl?: string;
  metadata?: Record<string, unknown>;
}

export interface OrgMemberInfoClass {
  orgId: string;
  orgName: string;
  orgMetadata?: Record<string, unknown>;
  userRole: string;
  userPermissions: string[];
}

interface MetaInfoContextType {
  user: UserClass;
  orgs: OrgMemberInfoClass[];
  activeOrg: OrgMemberInfoClass;
  setActiveOrg: (org: OrgMemberInfoClass) => void;
  switchOrganization: (org: OrgMemberInfoClass) => Promise<void>;
  accessToken: string;
  logout: () => Promise<void>;
  activeRole: OrganizationRole;
  toggleActiveRole: () => Promise<OrganizationRole>;
  isTokenRefreshing: boolean;
  // RBAC additions
  checkPermission: (permission: Permission) => boolean;
  getPermissions: () => readonly Permission[];
}

const MetaInfoContext = createContext<MetaInfoContextType | undefined>(undefined);

const MCP_BUNDLE_QUERY_KEY = ["mcp-server-bundles"] as const;

interface MetaInfoProviderProps {
  children: ReactNode;
}

export const MetaInfoProvider = ({ children }: MetaInfoProviderProps) => {
  const router = useRouter();
  const queryClient = useQueryClient();
  const [user, setUser] = useState<UserClass | null>(null);
  const [orgs, setOrgs] = useState<OrgMemberInfoClass[]>([]);
  const [activeOrg, setActiveOrg] = useState<OrgMemberInfoClass | null>(null);
  const [accessToken, setAccessToken] = useState<string>("");
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [isLoading, setIsLoading] = useState(true);
  const [activeRole, setActiveRole] = useState<OrganizationRole>(OrganizationRole.Admin);
  const [isTokenRefreshing, setIsTokenRefreshing] = useState(false);

  const handleLogout = useCallback(async () => {
    try {
      await apiLogout();
    } catch {
      // Continue with logout even if API call fails
    }

    // Clear ALL storage to prevent stale data
    if (typeof window !== "undefined") {
      localStorage.clear();
      sessionStorage.clear();
    }

    // Also explicitly clear managers (for safety/consistency)
    tokenManager.clearToken();
    roleManager.clearActiveRole();
    organizationManager.clearActiveOrganization();

    // Clear all state
    setAccessToken("");
    setUser(null);
    setOrgs([]);
    setActiveOrg(null);
    setIsAuthenticated(false);
    setActiveRole(OrganizationRole.Admin);

    // Redirect to home page after logout
    router.push("/");
  }, [router]);

  // Centralized token refresh function to avoid duplication
  const refreshTokenWithContext = useCallback(
    async (orgId: string, userRole: OrganizationRole) => {
      setIsTokenRefreshing(true);
      setAccessToken(""); // Clear immediately to prevent stale token usage

      try {
        tokenManager.clearToken();
        const newToken = await tokenManager.getAccessToken(orgId, userRole);

        if (newToken) {
          setAccessToken(newToken);
          return newToken;
        }
        throw new Error("Failed to refresh token");
      } catch (error) {
        console.error("Token refresh error:", error);
        // On error, redirect to login
        router.push("/login");
        throw error;
      } finally {
        setIsTokenRefreshing(false);
      }
    },
    [router],
  );

  // Check for existing session on mount
  useEffect(() => {
    const checkExistingSession = async () => {
      try {
        // IMPORTANT: Load stored preferences FIRST before requesting token
        // This ensures we have the correct act_as context from the start
        const storedOrg = organizationManager.getActiveOrganization();
        const storedRole = storedOrg ? roleManager.getActiveRole(storedOrg.orgId) : null;

        // Reset any stale act_as context from previous sessions before
        // requesting a fresh token.
        tokenManager.clearToken();

        // Always start by fetching a base token; we'll align org context after
        // verifying the stored preference still exists for this user.
        const token = await tokenManager.getAccessToken();

        if (token) {
          // Get user profile with the new token
          const userProfile = await getProfile(token);

          // Transform the profile to match the expected format
          const user: UserClass = {
            userId: userProfile.user_id,
            email: userProfile.email,
            firstName: userProfile.name.split(" ")[0],
            lastName: userProfile.name.split(" ").slice(1).join(" "),
            username: userProfile.email.split("@")[0],
            pictureUrl: `https://ui-avatars.com/api/?name=${encodeURIComponent(userProfile.name)}&background=random`,
          };
          // If the user has no organizations, send them to onboarding
          if (!userProfile.organizations || userProfile.organizations.length === 0) {
            setAccessToken(token);
            setUser(user);
            setOrgs([]);
            setActiveOrg(null);
            setIsAuthenticated(true);
            setIsLoading(false);
            router.push("/onboarding/organization");
            return;
          }

          const organizations = (userProfile.organizations ?? []).map((org) => ({
            orgId: org.organization_id,
            orgName: org.organization_name,
            userRole: org.role,
            userPermissions: [],
          }));

          let selectedOrg: OrgMemberInfoClass | null = null;
          const storedOrgMatch = storedOrg
            ? (organizations.find((org) => org.orgId === storedOrg.orgId) ?? null)
            : null;

          if (!storedOrgMatch && storedOrg) {
            organizationManager.clearActiveOrganization();
            roleManager.clearActiveRole();
          }

          if (storedOrgMatch) {
            selectedOrg = storedOrgMatch;
          } else if (organizations.length > 0) {
            selectedOrg = organizations[0];
          }

          setUser(user);
          setOrgs(organizations);

          if (selectedOrg) {
            setActiveOrg(selectedOrg);
            setIsAuthenticated(true);

            if (!storedOrg || storedOrg.orgId !== selectedOrg.orgId) {
              organizationManager.setActiveOrganization(
                selectedOrg.orgId,
                selectedOrg.orgName,
                selectedOrg.userRole,
              );
            }

            const storedRoleForSelectedOrg =
              storedRole && storedRole.organizationId === selectedOrg.orgId ? storedRole : null;

            if (
              selectedOrg.userRole === OrganizationRole.Admin &&
              storedRoleForSelectedOrg &&
              selectedOrg.orgId
            ) {
              setActiveRole(storedRoleForSelectedOrg.role);
            } else {
              setActiveRole(selectedOrg.userRole as OrganizationRole);
            }

            const expectedRoleForActAs =
              selectedOrg.userRole === OrganizationRole.Admin && storedRoleForSelectedOrg
                ? storedRoleForSelectedOrg.role
                : (selectedOrg.userRole as OrganizationRole);
            const currentActAs = tokenManager.getCurrentActAs();
            const needsOrgContextRefresh =
              !currentActAs ||
              currentActAs.organization_id !== selectedOrg.orgId ||
              currentActAs.role !== expectedRoleForActAs;

            if (needsOrgContextRefresh) {
              const refreshedToken = await refreshTokenWithContext(
                selectedOrg.orgId,
                selectedOrg.userRole as OrganizationRole,
              );

              if (!refreshedToken) {
                throw new Error("Failed to refresh token with organization context");
              }
            } else {
              setAccessToken(token);
            }
          } else {
            setActiveOrg(null);
            setIsAuthenticated(true);
            setAccessToken(token);
          }
        } else {
          // No valid session - this is expected for non-authenticated users
        }
      } catch (error) {
        console.error("Error checking session:", error);
      } finally {
        setIsLoading(false);
      }
    };

    checkExistingSession();
  }, [router, refreshTokenWithContext]);

  const switchOrganization = useCallback(
    async (org: OrgMemberInfoClass) => {
      // Save to localStorage
      organizationManager.setActiveOrganization(org.orgId, org.orgName, org.userRole);

      // Clear any role switching when changing organizations
      roleManager.clearActiveRole();
      setActiveRole(org.userRole as OrganizationRole);

      // Update state
      setActiveOrg(org);

      // Refresh token with loading state
      await refreshTokenWithContext(org.orgId, org.userRole as OrganizationRole);

      // Invalidate queries that depend on organization-bound auth context
      queryClient.invalidateQueries({
        queryKey: mcpQueryKeys.configurations.all,
      });
      queryClient.invalidateQueries({ queryKey: mcpQueryKeys.servers.all });
      queryClient.invalidateQueries({ queryKey: connectedAccountKeys.all() });
      queryClient.invalidateQueries({ queryKey: MCP_BUNDLE_QUERY_KEY });
    },
    [refreshTokenWithContext, queryClient],
  );

  const toggleActiveRole = useCallback(async () => {
    if (!activeOrg || activeOrg.userRole !== OrganizationRole.Admin) {
      return activeRole; // Only admins can toggle role
    }

    // Determine new role state - toggle between Admin and Member
    const newActiveRole =
      activeRole === OrganizationRole.Admin ? OrganizationRole.Member : OrganizationRole.Admin;

    // Update localStorage first
    if (newActiveRole === OrganizationRole.Member) {
      roleManager.setActiveRole(activeOrg.orgId, OrganizationRole.Member);
    } else {
      roleManager.clearActiveRole();
    }

    // Update state optimistically
    setActiveRole(newActiveRole);

    // Refresh token with the correct context
    // The tokenManager will check roleManager internally for act_as context
    await refreshTokenWithContext(activeOrg.orgId, activeOrg.userRole as OrganizationRole);

    // Invalidate queries that depend on organization-bound auth context
    queryClient.invalidateQueries({
      queryKey: mcpQueryKeys.configurations.all,
    });
    queryClient.invalidateQueries({ queryKey: mcpQueryKeys.servers.all });
    queryClient.invalidateQueries({ queryKey: connectedAccountKeys.all() });
    queryClient.invalidateQueries({ queryKey: MCP_BUNDLE_QUERY_KEY });

    // Return the new state so callers can use it immediately
    return newActiveRole;
  }, [activeOrg, activeRole, refreshTokenWithContext, queryClient]);

  // RBAC helper functions
  const checkPermissionCallback = useCallback(
    (permission: Permission): boolean => {
      if (!activeOrg) return false;
      // Use the active role for permission checking
      const roleToCheck = activeRole;
      return checkPermission(roleToCheck.toLowerCase(), permission);
    },
    [activeOrg, activeRole],
  );

  const getPermissionsCallback = useCallback((): readonly Permission[] => {
    if (!activeOrg) return [];
    // Use the active role for permission checking
    const roleToCheck = activeRole;
    return getPermissionsForRole(roleToCheck.toLowerCase());
  }, [activeOrg, activeRole]);

  useEffect(() => {
    if (orgs.length > 0 && !activeOrg) {
      // Set the first org as active if none is set
      const orgToSet = orgs[0];
      setActiveOrg(orgToSet);
      // Save to localStorage
      organizationManager.setActiveOrganization(
        orgToSet.orgId,
        orgToSet.orgName,
        orgToSet.userRole,
      );
    }
  }, [orgs, activeOrg]);

  // Redirect to login page if not authenticated
  useEffect(() => {
    if (!isLoading && !isAuthenticated) {
      router.push("/login");
    }
  }, [isLoading, isAuthenticated, router]);

  // Show loading while checking existing session
  if (isLoading) {
    return (
      <div className="flex min-h-screen flex-col items-center justify-center space-y-3">
        <h1 className="text-2xl font-semibold">Loading...</h1>
        <Skeleton className="h-[125px] w-[250px] rounded-xl" />
      </div>
    );
  }

  // Show loading while redirecting to login
  if (!isAuthenticated) {
    return (
      <div className="flex min-h-screen flex-col items-center justify-center space-y-3">
        <h1 className="text-2xl font-semibold">Redirecting to login...</h1>
        <Skeleton className="h-[125px] w-[250px] rounded-xl" />
      </div>
    );
  }

  // Show loading while fetching initial data
  if (!user || !activeOrg || !accessToken) {
    return (
      <div className="flex min-h-screen flex-col items-center justify-center space-y-3">
        <h1 className="text-2xl font-semibold">Setting up your workspace...</h1>
        <Skeleton className="h-[125px] w-[250px] rounded-xl" />
        <div className="space-y-2">
          <Skeleton className="h-4 w-[250px]" />
          <Skeleton className="h-4 w-[200px]" />
        </div>
      </div>
    );
  }

  return (
    <MetaInfoContext.Provider
      value={{
        user,
        orgs,
        activeOrg,
        setActiveOrg,
        switchOrganization,
        accessToken,
        logout: handleLogout,
        activeRole,
        toggleActiveRole,
        isTokenRefreshing,
        // RBAC additions
        checkPermission: checkPermissionCallback,
        getPermissions: getPermissionsCallback,
      }}
    >
      {children}
    </MetaInfoContext.Provider>
  );
};

export const useMetaInfo = () => {
  const context = useContext(MetaInfoContext);
  if (!context) {
    throw new Error("useMetaInfo must be used within a MetaInfoProvider");
  }
  return context;
};
