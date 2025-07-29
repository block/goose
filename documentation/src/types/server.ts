export interface MCPServer {
  id: string;
  name: string;
  description: string;
  command?: string;
  url?: string;
  type?: "local" | "remote" | "streamable-http";
  link: string;
  installation_notes: string;
  is_builtin: boolean;
  endorsed: boolean;
  environmentVariables: {
    name: string;
    description: string;
    required: boolean;
  }[];
}