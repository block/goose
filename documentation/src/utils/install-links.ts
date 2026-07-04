import type { MCPServer } from "../types/server";

export function getGoslingInstallLink(server: MCPServer): string {
  if (server.is_builtin) {
    const queryParams = [
      'cmd=gosling',
      'arg=mcp',
      `arg=${encodeURIComponent(server.id)}`,
      `description=${encodeURIComponent(server.id)}`
    ].join('&');
    return `gosling://extension?${queryParams}`;
  }

  // Handle the case where the command is a URL
  if (server.url) {
    const queryParams = [
      // Map the type to the expected format for the deep link
      ...(server.type === "streamable-http" ? [`type=streamable_http`] : []),
      `url=${encodeURIComponent(server.url)}`,
      `id=${encodeURIComponent(server.id)}`,
      `name=${encodeURIComponent(server.name)}`,
      `description=${encodeURIComponent(server.description)}`,
      ...server.environmentVariables
        .filter((env) => env.required)
        .map(
          (env) => `env=${encodeURIComponent(`${env.name}=${env.description}`)}`
        ),
      ...(server.headers || [])
        .filter((header) => header.required)
        .map(
          (header) => `header=${encodeURIComponent(`${header.name}=${header.description}`)}`
        ),
    ].join("&");
  
    return `gosling://extension?${queryParams}`;
  }
  
  const parts = server.command.split(" ");
  const baseCmd = parts[0]; // docker, jbang, npx or uvx
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

  return `gosling://extension?${queryParams}`;
}
