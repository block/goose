export enum OrganizationRole {
  Admin = "admin",
  Member = "member",
}

export interface OrganizationUser {
  user_id: string;
  email: string;
  role: string; // Will be "admin" or "member" from backend
  name: string; // Full name from backend
  created_at?: string;
  // Frontend display fields (parsed from name)
  first_name?: string;
  last_name?: string;
}

export interface OrganizationMemberUpdate {
  role: OrganizationRole;
}
