import type {
  McpUiToolInputNotification,
  McpUiToolInputPartialNotification,
  McpUiToolCancelledNotification,
  McpUiDisplayMode,
} from '@modelcontextprotocol/ext-apps/app-bridge';

import type {
  CreateMessageRequest,
  CreateMessageResult,
} from '@modelcontextprotocol/sdk/types.js';

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

// ============================================================================
// MCP Request Handler Types
// ============================================================================
// These types support a generic request handler for MCP methods that the SDK's
// AppRenderer doesn't handle natively. Once the SDK adds an `onRequest` prop,
// we can use these types to handle custom methods like `sampling/createMessage`.

/**
 * Map of MCP method names to their request params.
 * Add new methods here as needed.
 */
export interface McpRequestParams {
  'sampling/createMessage': CreateMessageRequest['params'];
}

/**
 * Map of MCP method names to their response types.
 * Add new methods here as needed.
 */
export interface McpRequestResult {
  'sampling/createMessage': CreateMessageResult;
}

/**
 * Generic handler for MCP requests not handled by the SDK's AppRenderer.
 * This allows Goose to handle custom methods like `sampling/createMessage`.
 *
 * @example
 * ```typescript
 * const handleRequest: McpRequestHandler = async (method, params) => {
 *   switch (method) {
 *     case 'sampling/createMessage':
 *       return await handleSamplingRequest(params);
 *     default:
 *       throw new Error(`Unhandled method: ${method}`);
 *   }
 * };
 * ```
 */
export type McpRequestHandler = <M extends keyof McpRequestParams>(
  method: M,
  params: McpRequestParams[M]
) => Promise<McpRequestResult[M]>;
