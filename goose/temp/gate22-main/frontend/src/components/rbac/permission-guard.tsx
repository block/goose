"use client";

import React from "react";
import { usePermissions } from "@/hooks/use-permissions";
import { Permission } from "@/lib/rbac/permissions";

interface PermissionGuardProps {
  permission: Permission | Permission[];
  mode?: "all" | "any";
  fallback?: React.ReactNode;
  children: React.ReactNode;
}

/**
 * Component that conditionally renders children based on user permissions
 *
 * @example
 * ```tsx
 * <PermissionGuard permission={PERMISSIONS.MCP_CONFIGURATION_CREATE}>
 *   <Button>Create Configuration</Button>
 * </PermissionGuard>
 * ```
 *
 * @example Multiple permissions with 'any' mode
 * ```tsx
 * <PermissionGuard
 *   permission={[PERMISSIONS.BUNDLE_CREATE, PERMISSIONS.BUNDLE_DELETE_OWN]}
 *   mode="any"
 * >
 *   <Button>Manage Bundle</Button>
 * </PermissionGuard>
 * ```
 */
export function PermissionGuard({
  permission,
  mode = "all",
  fallback = null,
  children,
}: PermissionGuardProps) {
  const permissions = Array.isArray(permission) ? permission : [permission];
  const hasPermission = usePermissions(permissions, mode);

  if (!hasPermission) {
    return <>{fallback}</>;
  }

  return <>{children}</>;
}

/**
 * Higher-order component version of PermissionGuard for easier composition
 */
export function withPermissionGuard<P extends object>(
  permission: Permission | Permission[],
  mode: "all" | "any" = "all",
  fallback?: React.ReactNode,
) {
  return function WithPermissionGuardWrapper(Component: React.ComponentType<P>) {
    return function WithPermissionGuardComponent(props: P) {
      return (
        <PermissionGuard permission={permission} mode={mode} fallback={fallback}>
          <Component {...props} />
        </PermissionGuard>
      );
    };
  };
}
