export interface MCPServer {
  id: string;
  name: string;
  description: string;
  githubStars: number;
  command: string;
  environmentVariables: {
    name: string;
    description: string;
    required: boolean;
  }[];
}