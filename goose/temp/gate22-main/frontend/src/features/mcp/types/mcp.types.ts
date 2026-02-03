export enum AuthType {
  OAUTH = "oauth2",
  API_KEY = "api_key",
  NO_AUTH = "no_auth",
}

export enum ConnectedAccountOwnership {
  INDIVIDUAL = "individual",
  SHARED = "shared",
  OPERATIONAL = "operational",
}

export interface AuthConfig {
  type: AuthType;
  [key: string]: unknown;
}

// Tool without schema (used in server responses to keep payload small)
export interface MCPToolBasic {
  id: string;
  name: string;
  description: string;
  tags: string[];
  created_at: string;
  updated_at: string;
}

// Full tool with schema (fetched individually)
export interface MCPToolPublic extends MCPToolBasic {
  input_schema: Record<string, unknown>;
}

export interface MCPServerPublicBasic {
  id: string;
  name: string;
  url: string;
  description: string;
  logo: string;
  categories: string[];
}

export interface MCPServerPublic extends MCPServerPublicBasic {
  supported_auth_types: AuthType[];
  tools: MCPToolBasic[];
  created_at: string;
  updated_at: string;
  last_synced_at: string | null;
  organization_id: string | null;
}

export interface TeamInfo {
  team_id: string;
  name: string;
  description?: string;
  created_at: string;
}

export interface MCPServerConfigurationCreate {
  mcp_server_id: string;
  name: string;
  description?: string;
  auth_type: AuthType;
  connected_account_ownership: ConnectedAccountOwnership;
  all_tools_enabled: boolean;
  enabled_tools: string[];
  allowed_teams: string[];
}

export interface MCPServerConfigurationUpdate {
  name?: string;
  description?: string;
  all_tools_enabled?: boolean;
  enabled_tools?: string[];
  allowed_teams?: string[];
}

export interface MCPServerConfigurationPublic {
  id: string;
  mcp_server_id: string;
  organization_id: string;
  name: string;
  description?: string;
  auth_type: AuthType;
  connected_account_ownership: ConnectedAccountOwnership;
  all_tools_enabled: boolean;
  enabled_tools: MCPToolBasic[];
  allowed_teams: TeamInfo[];
  created_at: string;
  updated_at: string;
  mcp_server: MCPServerPublicBasic;
  has_operational_connected_account?: boolean;
}

export interface MCPServerConfigurationPublicBasic {
  id: string;
  mcp_server_id: string;
  name: string;
  description?: string;
  connected_account_ownership?: ConnectedAccountOwnership;
  mcp_server: MCPServerPublicBasic;
  created_at?: string;
  updated_at?: string;
}

export interface PaginationParams {
  offset?: number;
  limit?: number;
}

export interface PaginationResponse<T> {
  data: T[];
  offset: number;
  total?: number;
}

export interface ToolsSyncResult {
  tools_created: string[];
  tools_deleted: string[];
  tools_updated: string[];
  tools_unchanged: string[];
}
