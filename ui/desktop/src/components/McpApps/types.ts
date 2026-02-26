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
 * Per the ext-apps spec, each axis is independently:
 *   fixed     – sends width/height (host controls, view fills it)
 *   flexible  – sends maxWidth/maxHeight (view controls, up to max; host resizes via size-changed)
 *   unbounded – field omitted (view controls with no limit; host resizes via size-changed)
 */
export type DimensionMode = 'fixed' | 'flexible' | 'unbounded';

export interface DimensionLayout {
  width: DimensionMode;
  height: DimensionMode;
}

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
  _meta?: { [key: string]: unknown };
};

export type SamplingMessage = {
  role: 'user' | 'assistant';
  content: { type: 'text'; text: string } | { type: 'image'; data: string; mimeType: string };
};

export type SamplingCreateMessageParams = {
  messages: SamplingMessage[];
  systemPrompt?: string;
  maxTokens?: number;
};

export type SamplingCreateMessageResponse = {
  model: string;
  stopReason: string;
  role: 'assistant';
  content: { type: 'text'; text: string };
};
