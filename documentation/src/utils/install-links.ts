import type { MCPServer } from "../types/server";

export function getGooseInstallLink(server: MCPServer): string {
  return `goose://install/${encodeURIComponent(server.command)}`;
}