import { MCPToolBasic, TeamInfo } from "@/features/mcp/types/mcp.types";

export interface MCPServerBundle {
  id: string;
  name: string;
  description?: string | null;
  user_id: string;
  organization_id: string;
  mcp_server_configurations: MCPServerConfiguration[];
  user?: {
    id: string;
    name: string;
    email: string;
  };
  bundle_key?: string;
  created_at: string;
  updated_at: string;
}

export interface MCPServerConfiguration {
  id: string;
  name: string;
  mcp_server_id: string;
  organization_id: string;
  auth_type: string;
  all_tools_enabled: boolean;
  enabled_tools: MCPToolBasic[]; // Array of tool objects with id, name, description
  allowed_teams: TeamInfo[]; // Array of team objects with team_id, name, description
  created_at: string;
  updated_at: string;
  mcp_server: {
    id: string;
    name: string;
    description?: string | null;
    logo?: string | null;
    icon_url?: string | null;
    categories?: string[];
  };
}

// MCPServerBundleDetailed is now the same as MCPServerBundle
// since MCPServerBundle already includes mcp_server_configurations
export type MCPServerBundleDetailed = MCPServerBundle;

export interface CreateMCPServerBundleInput {
  name: string;
  description?: string | null;
  mcp_server_configuration_ids: string[];
}

export interface UpdateMCPServerBundleInput {
  name?: string;
  description?: string | null;
  mcp_server_configuration_ids?: string[];
}
