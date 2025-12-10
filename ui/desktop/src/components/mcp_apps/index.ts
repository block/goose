/**
 * MCP Apps Components (SEP-1865)
 *
 * This module contains components for rendering MCP Apps - interactive UI
 * resources from MCP servers that follow the SEP-1865 specification.
 */

export {
  default as McpAppRenderer,
  isMcpApp,
  MCP_APPS_MIME_TYPE,
  MCP_APPS_URI_SCHEME,
  MCP_APPS_METHODS,
} from './McpAppRenderer';

export type { ToolInputData, ToolResultData, ToolCancelledData } from './McpAppRenderer';
