import { OrganizationRole } from "@/features/settings/types/organization.types";

interface StoredRole {
  organizationId: string;
  role: OrganizationRole;
}

class RoleManager {
  private static instance: RoleManager;
  private readonly STORAGE_KEY = "activeRole";

  private constructor() {}

  static getInstance(): RoleManager {
    if (!RoleManager.instance) {
      RoleManager.instance = new RoleManager();
    }
    return RoleManager.instance;
  }

  getActiveRole(organizationId: string): StoredRole | null {
    if (typeof window === "undefined") return null;

    try {
      const stored = localStorage.getItem(this.STORAGE_KEY);
      if (!stored) return null;

      const parsedRole = JSON.parse(stored) as StoredRole;

      if (parsedRole.organizationId !== organizationId) {
        return null;
      }

      return parsedRole;
    } catch {
      return null;
    }
  }

  setActiveRole(organizationId: string, role: OrganizationRole): void {
    if (typeof window === "undefined") return;

    const roleData: StoredRole = {
      organizationId,
      role,
    };

    localStorage.setItem(this.STORAGE_KEY, JSON.stringify(roleData));
  }

  clearActiveRole(): void {
    if (typeof window === "undefined") return;
    localStorage.removeItem(this.STORAGE_KEY);
  }
}

export const roleManager = RoleManager.getInstance();
