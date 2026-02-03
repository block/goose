interface StoredOrganization {
  orgId: string;
  orgName: string;
  userRole: string;
}

class OrganizationManager {
  private static instance: OrganizationManager;
  private readonly STORAGE_KEY = "activeOrganization";

  private constructor() {}

  static getInstance(): OrganizationManager {
    if (!OrganizationManager.instance) {
      OrganizationManager.instance = new OrganizationManager();
    }
    return OrganizationManager.instance;
  }

  getActiveOrganization(): StoredOrganization | null {
    if (typeof window === "undefined") return null;

    try {
      const stored = localStorage.getItem(this.STORAGE_KEY);
      if (!stored) return null;

      return JSON.parse(stored) as StoredOrganization;
    } catch {
      return null;
    }
  }

  setActiveOrganization(orgId: string, orgName: string, userRole: string): void {
    if (typeof window === "undefined") return;

    const orgData: StoredOrganization = {
      orgId,
      orgName,
      userRole,
    };

    localStorage.setItem(this.STORAGE_KEY, JSON.stringify(orgData));
  }

  clearActiveOrganization(): void {
    if (typeof window === "undefined") return;
    localStorage.removeItem(this.STORAGE_KEY);
  }
}

export const organizationManager = OrganizationManager.getInstance();
