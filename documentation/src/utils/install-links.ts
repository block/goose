import type { MCPServer } from "../types/server";

export function getGooseInstallLink(server: MCPServer): string {
  if (server.is_builtin) {
    const queryParams = [
      'cmd=goosed',
      'arg=mcp',
      `arg=${encodeURIComponent(server.id)}`,
      `description=${encodeURIComponent(server.id)}`
    ].join('&');
    return `goose://extension?${queryParams}`;
  }
  const parts = server.command.split(" ");
  const baseCmd = parts[0]; // npx or uvx
  const args = parts.slice(1); // remaining arguments
  const queryParams = [
    `cmd=${encodeURIComponent(baseCmd)}`,
    ...args.map((arg) => `arg=${encodeURIComponent(arg)}`),
    `id=${encodeURIComponent(server.id)}`,
    `name=${encodeURIComponent(server.name)}`,
    `description=${encodeURIComponent(server.description)}`,
    ...server.environmentVariables
      .filter((env) => env.required)
      .map(
        (env) => `env=${encodeURIComponent(`${env.name}=${env.description}`)}`
      ),
  ].join("&");

  return `goose://extension?${queryParams}`;
}