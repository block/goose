export interface TeamMember {
  user_id: string;
  name: string;
  email: string;
  role: string;
  created_at: string;
}

export interface Team {
  team_id: string;
  name: string;
  description?: string;
  member_count?: number;
  status?: "active" | "inactive";
  created_at: string;
}

export interface CreateTeamRequest {
  name: string;
  description?: string;
  member_user_ids?: string[];
}
