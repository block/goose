import type {
  McpUiToolInputNotification,
  McpUiToolInputPartialNotification,
  McpUiToolCancelledNotification,
  McpUiDisplayMode,
} from '@modelcontextprotocol/ext-apps/app-bridge';
import type { Content } from '../../api';

/**
 * Space-separated sandbox tokens for iframe permissions.
 * @see https://developer.mozilla.org/en-US/docs/Web/HTML/Element/iframe#sandbox
 */
export type SandboxPermissions = string;

export type GooseDisplayMode = McpUiDisplayMode | 'standalone';

/**
 * Tool input from the message stream.
 * McpAppRenderer extracts `.arguments` when passing to the SDK's AppRenderer.
 */
export type McpAppToolInput = McpUiToolInputNotification['params'];

export type McpAppToolInputPartial = McpUiToolInputPartialNotification['params'];

export type McpAppToolCancelled = McpUiToolCancelledNotification['params'];

export type McpAppToolResult = {
  content: Content[];
  structuredContent?: unknown;
};
