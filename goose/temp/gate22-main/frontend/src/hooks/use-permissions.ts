import { useMemo } from "react";
import { useMetaInfo } from "@/components/context/metainfo";
import {
  checkPermission,
  checkMultiplePermissions,
  getPermissionsForRole,
} from "@/lib/rbac/rbac-service";
import { Permission } from "@/lib/rbac/permissions";
import { OrganizationRole } from "@/features/settings/types/organization.types";

/**
 * Hook to check if current user has a specific permission
 */
export function usePermission(permission: Permission): boolean {
  const { activeOrg, activeRole } = useMetaInfo();

  return useMemo(() => {
    if (!activeOrg) return false;

    // Use the active role for permission checking
    const roleToCheck = activeRole;

    return checkPermission(roleToCheck.toLowerCase(), permission);
  }, [activeOrg, activeRole, permission]);
}

/**
 * Hook to check if current user has multiple permissions
 */
export function usePermissions(permissions: Permission[], mode: "all" | "any" = "all"): boolean {
  const { activeOrg, activeRole } = useMetaInfo();

  return useMemo(() => {
    if (!activeOrg) return false;

    // Use the active role for permission checking
    const roleToCheck = activeRole;

    return checkMultiplePermissions(roleToCheck.toLowerCase(), permissions, mode);
  }, [activeOrg, activeRole, permissions, mode]);
}

/**
 * Hook to get current user's role information
 */
export function useRole(): {
  role: string;
  activeRole: string;
  isAdmin: boolean;
  isMember: boolean;
  isActingAsMember: boolean;
} {
  const { activeOrg, activeRole } = useMetaInfo();

  return useMemo(() => {
    const userRole = activeOrg?.userRole || "";
    const effectiveRole = activeRole ? activeRole.toLowerCase() : userRole.toLowerCase();

    return {
      role: userRole.toLowerCase(),
      activeRole: effectiveRole,
      isAdmin: userRole === OrganizationRole.Admin,
      isMember: userRole === OrganizationRole.Member,
      isActingAsMember:
        userRole === OrganizationRole.Admin && activeRole === OrganizationRole.Member,
    };
  }, [activeOrg, activeRole]);
}

/**
 * Hook to get all permissions for the current user
 */
export function useUserPermissions(): readonly Permission[] {
  const { activeOrg, activeRole } = useMetaInfo();

  return useMemo(() => {
    if (!activeOrg) return [];

    // Use the active role for permission checking
    const roleToCheck = activeRole;

    return getPermissionsForRole(roleToCheck.toLowerCase());
  }, [activeOrg, activeRole]);
}

/**
 * Hook to check if user can perform an action on a resource
 * Considers ownership for resource-specific actions
 */
export function useCanPerformAction(action: Permission, resourceOwnerId?: string): boolean {
  const { activeOrg, activeRole, user } = useMetaInfo();

  return useMemo(() => {
    if (!activeOrg || !user) return false;

    // Use the active role for permission checking
    const roleToCheck = activeRole;

    // Check base permission first
    const hasPermission = checkPermission(roleToCheck.toLowerCase(), action);

    // For ownership-based permissions, check if user owns the resource
    // This is a simplified check - you may want to expand this based on your needs
    if (hasPermission && resourceOwnerId) {
      // If action requires ownership (e.g., delete_own), verify ownership
      if (action.includes("_own")) {
        return resourceOwnerId === user.userId;
      }
    }

    return hasPermission;
  }, [activeOrg, activeRole, user, action, resourceOwnerId]);
}
