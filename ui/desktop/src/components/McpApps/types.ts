/**
 * Iframe sandbox attribute tokens.
 * @see https://developer.mozilla.org/en-US/docs/Web/HTML/Element/iframe#sandbox
 */
export type SandboxToken =
  | 'allow-downloads'
  | 'allow-forms'
  | 'allow-modals'
  | 'allow-orientation-lock'
  | 'allow-pointer-lock'
  | 'allow-popups'
  | 'allow-popups-to-escape-sandbox'
  | 'allow-presentation'
  | 'allow-same-origin'
  | 'allow-scripts'
  | 'allow-top-navigation'
  | 'allow-top-navigation-by-user-activation'
  | 'allow-top-navigation-to-custom-protocols';

/** Space-separated sandbox tokens for iframe permissions */
export type SandboxPermissions = string;

/**
 * Wrapper for tool arguments from the message stream.
 * McpAppRenderer extracts `.arguments` when passing to the SDK's AppRenderer.
 */
export interface ToolInput {
  arguments: Record<string, unknown>;
}

export interface ToolInputPartial {
  arguments: Record<string, unknown>;
}

export interface ToolCancelled {
  reason?: string;
}

/** Content block for MCP messages */
export type ContentBlock = { type: 'text'; text: string } | { type: 'image'; data: string };
