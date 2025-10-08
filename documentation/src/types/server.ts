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
  show_install_link?: boolean;
  show_install_command?: boolean;
  environmentVariables: {
    name: string;
    description: string;
    required: boolean;
  }[];
  headers?: {
    name: string;
    description: string;
    required: boolean;
  }[];
}