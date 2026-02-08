import type {
  McpUiToolInputNotification,
  McpUiToolInputPartialNotification,
  McpUiToolCancelledNotification,
  McpUiDisplayMode,
} from '@modelcontextprotocol/ext-apps/app-bridge';
import { Content } from '../../api';

/**
 * Space-separated sandbox tokens for iframe permissions.
 * @see https://developer.mozilla.org/en-US/docs/Web/HTML/Element/iframe#sandbox
 */
export type SandboxPermissions = string;

/**
 * Display modes for MCP apps in Goose.
 *
 * Extends the SDK's McpUiDisplayMode with Goose-specific modes:
 * - `inline`: Embedded in chat flow (default)
 * - `fullscreen`: Takes over the current Goose window with close button
 * - `pip`: Picture-in-picture floating window
 * - `standalone`: Rendered in a separate Electron window
 */
export type GooseDisplayMode = McpUiDisplayMode | 'standalone';

/**
 * Tool input from the message stream.
 * McpAppRenderer extracts `.arguments` when passing to the SDK's AppRenderer.
 */
export type ToolInput = McpUiToolInputNotification['params'];

export type ToolInputPartial = McpUiToolInputPartialNotification['params'];

export type ToolCancelled = McpUiToolCancelledNotification['params'];

export type ToolResult = {
  content: Content[];
  structuredContent?: unknown;
};
