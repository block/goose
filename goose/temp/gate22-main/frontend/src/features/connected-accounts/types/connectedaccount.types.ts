import { MCPServerConfigurationBasic } from "./mcp-server-configuration.types";
import { ConnectedAccountOwnership } from "@/features/mcp/types/mcp.types";

export type ConnectedAccount = {
  id: string;
  user_id: string;
  mcp_server_configuration_id: string;
  ownership: ConnectedAccountOwnership;
  created_at: string;
  updated_at: string;
  last_used_at?: string;
  mcp_server_configuration: MCPServerConfigurationBasic;
  user?: {
    id: string;
    name: string;
    email: string;
  };
};
