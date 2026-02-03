export type MCPServerBasic = {
  id: string;
  name: string;
  description?: string;
};

export type MCPServerConfigurationBasic = {
  id: string;
  mcp_server_id: string;
  mcp_server: MCPServerBasic;
};
