import type {
  McpUiToolInputNotification,
  McpUiToolInputPartialNotification,
  McpUiToolCancelledNotification,
} from '@modelcontextprotocol/ext-apps/app-bridge';

/**
 * Space-separated sandbox tokens for iframe permissions.
 * @see https://developer.mozilla.org/en-US/docs/Web/HTML/Element/iframe#sandbox
 */
export type SandboxPermissions = string;

/**
 * Tool input from the message stream.
 * McpAppRenderer extracts `.arguments` when passing to the SDK's AppRenderer.
 */
export type ToolInput = McpUiToolInputNotification['params'];

export type ToolInputPartial = McpUiToolInputPartialNotification['params'];

export type ToolCancelled = McpUiToolCancelledNotification['params'];
