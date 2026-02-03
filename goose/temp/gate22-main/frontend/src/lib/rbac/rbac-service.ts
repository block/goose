import { ROLE_PERMISSIONS, Permission } from "./permissions";

/**
 * Check if a role has a specific permission
 */
export function checkPermission(role: string, permission: Permission): boolean {
  const permissions = ROLE_PERMISSIONS[role.toLowerCase()];
  if (!permissions) {
    console.warn(`Unknown role: ${role}`);
    return false;
  }
  return permissions.includes(permission);
}

/**
 * Check if a role has multiple permissions
 * @param mode 'all' requires all permissions, 'any' requires at least one
 */
export function checkMultiplePermissions(
  role: string,
  permissions: Permission[],
  mode: "all" | "any" = "all",
): boolean {
  if (permissions.length === 0) return true;

  if (mode === "all") {
    return permissions.every((permission) => checkPermission(role, permission));
  } else {
    return permissions.some((permission) => checkPermission(role, permission));
  }
}

/**
 * Get all permissions for a role
 */
export function getPermissionsForRole(role: string): readonly Permission[] {
  const permissions = ROLE_PERMISSIONS[role.toLowerCase()];
  if (!permissions) {
    console.warn(`Unknown role: ${role}`);
    return [];
  }
  return permissions;
}

/**
 * Check if a role has any of the specified permissions
 */
export function hasAnyPermission(role: string, permissions: Permission[]): boolean {
  return checkMultiplePermissions(role, permissions, "any");
}

/**
 * Check if a role has all of the specified permissions
 */
export function hasAllPermissions(role: string, permissions: Permission[]): boolean {
  return checkMultiplePermissions(role, permissions, "all");
}
