/**
 * Utility for detecting long-running "run" commands in tool calls
 * 
 * These are commands that typically start dev servers or watch processes
 * that run indefinitely until manually stopped. When these are running,
 * we should allow the user to continue chatting without queuing.
 */

import { Message, MessageContent, ToolRequest } from '../api';

/**
 * Patterns that indicate a long-running "run" command
 * These commands typically start servers or watch processes
 */
const RUN_COMMAND_PATTERNS: RegExp[] = [
  // npm/yarn/pnpm run commands
  /\b(npm|yarn|pnpm)\s+(run\s+)?(dev|start|serve|watch|preview)\b/i,
  /\b(npm|yarn|pnpm)\s+run\s+\S+/i, // Any npm run <script>
  
  // Direct node/ts-node execution
  /\bnode\s+.*\.(js|mjs|cjs)\b/i,
  /\bts-node\s+/i,
  /\btsx\s+/i,
  /\bnpx\s+(ts-node|tsx|vite|next|nuxt|astro)\b/i,
  
  // Common dev servers
  /\b(vite|next|nuxt|astro|remix|gatsby)\s+(dev|start)?\b/i,
  /\buvicorn\s+/i,
  /\bflask\s+run\b/i,
  /\bdjango.*runserver\b/i,
  /\brails\s+s(erver)?\b/i,
  /\bphp\s+(-S|artisan\s+serve)\b/i,
  
  // Rust/Go/other
  /\bcargo\s+(run|watch)\b/i,
  /\bgo\s+run\b/i,
  
  // Watch/build commands
  /\b(tsc|webpack|rollup|esbuild|parcel)\s+.*(-w|--watch)\b/i,
  /\bnodemon\s+/i,
  /\bpm2\s+start\b/i,
  
  // Docker
  /\bdocker(-compose)?\s+(up|run)\b/i,
  
  // Generic patterns
  /\b(serve|server|dev|start)\s*$/i, // Commands ending with these words
  /--watch\b/i,
  /--hot\b/i,
];

/**
 * Commands that are explicitly NOT long-running (quick operations)
 * These take precedence over run patterns
 */
const QUICK_COMMAND_PATTERNS: RegExp[] = [
  /\b(npm|yarn|pnpm)\s+install\b/i,
  /\b(npm|yarn|pnpm)\s+(add|remove|uninstall)\b/i,
  /\bgit\s+(status|log|diff|add|commit|push|pull|fetch|checkout|branch)\b/i,
  /\bls\b/i,
  /\bcat\b/i,
  /\bgrep\b/i,
  /\bfind\b/i,
  /\bmkdir\b/i,
  /\brm\b/i,
  /\bcp\b/i,
  /\bmv\b/i,
  /\becho\b/i,
  /\bpwd\b/i,
  /\bcd\b/i,
  /\bwhich\b/i,
  /\bwhereis\b/i,
];

/**
 * Check if a shell command is a long-running "run" command
 */
export function isRunCommand(command: string): boolean {
  if (!command || command.trim().length === 0) {
    return false;
  }

  const normalizedCommand = command.trim();

  // First check if it's a quick command (takes precedence)
  for (const pattern of QUICK_COMMAND_PATTERNS) {
    if (pattern.test(normalizedCommand)) {
      return false;
    }
  }

  // Then check if it matches a run command pattern
  for (const pattern of RUN_COMMAND_PATTERNS) {
    if (pattern.test(normalizedCommand)) {
      return true;
    }
  }

  return false;
}

/**
 * Extract the shell command from a tool call if it's a shell tool
 */
function getShellCommandFromToolCall(toolCall: Record<string, unknown>): string | null {
  // Handle wrapped format: { status: "success", value: { name, arguments } }
  const unwrapped = toolCall?.status === 'success' 
    ? (toolCall.value as { name: string; arguments?: Record<string, unknown> })
    : (toolCall as { name: string; arguments?: Record<string, unknown> });

  if (!unwrapped?.name) {
    return null;
  }

  // Check if this is a shell tool (ends with __shell or is just "shell")
  const toolName = unwrapped.name;
  const isShellTool = toolName === 'shell' || toolName.endsWith('__shell');

  if (!isShellTool) {
    return null;
  }

  // Get the command argument
  const args = unwrapped.arguments;
  if (args && typeof args.command === 'string') {
    return args.command;
  }

  return null;
}

/**
 * Check if a message content item is a running tool request with a "run" command
 */
function isRunningRunCommand(content: MessageContent): boolean {
  if (content.type !== 'toolRequest') {
    return false;
  }

  const toolRequest = content as ToolRequest & { type: 'toolRequest' };
  const command = getShellCommandFromToolCall(toolRequest.toolCall);
  
  if (!command) {
    return false;
  }

  return isRunCommand(command);
}

/**
 * Check if there's an active "run" command in the current messages
 * 
 * A "run" command is considered active if:
 * 1. There's a toolRequest for a shell command matching run patterns
 * 2. There's no corresponding toolResponse yet (still running)
 */
export function hasActiveRunCommand(messages: Message[]): boolean {
  if (!messages || messages.length === 0) {
    return false;
  }

  // Collect all tool request IDs that have responses
  const respondedToolIds = new Set<string>();
  
  for (const message of messages) {
    for (const content of message.content) {
      if (content.type === 'toolResponse') {
        const toolResponse = content as { type: 'toolResponse'; id: string };
        respondedToolIds.add(toolResponse.id);
      }
    }
  }

  // Check for tool requests that are "run" commands without responses
  for (const message of messages) {
    for (const content of message.content) {
      if (content.type === 'toolRequest') {
        const toolRequest = content as ToolRequest & { type: 'toolRequest' };
        
        // Skip if this tool already has a response
        if (respondedToolIds.has(toolRequest.id)) {
          continue;
        }

        // Check if this is a run command
        if (isRunningRunCommand(content)) {
          return true;
        }
      }
    }
  }

  return false;
}

/**
 * Get details about the active run command (if any)
 */
export interface ActiveRunCommandInfo {
  command: string;
  toolId: string;
  toolName: string;
}

export function getActiveRunCommand(messages: Message[]): ActiveRunCommandInfo | null {
  if (!messages || messages.length === 0) {
    return null;
  }

  // Collect all tool request IDs that have responses
  const respondedToolIds = new Set<string>();
  
  for (const message of messages) {
    for (const content of message.content) {
      if (content.type === 'toolResponse') {
        const toolResponse = content as { type: 'toolResponse'; id: string };
        respondedToolIds.add(toolResponse.id);
      }
    }
  }

  // Find the first active run command
  for (const message of messages) {
    for (const content of message.content) {
      if (content.type === 'toolRequest') {
        const toolRequest = content as ToolRequest & { type: 'toolRequest' };
        
        // Skip if this tool already has a response
        if (respondedToolIds.has(toolRequest.id)) {
          continue;
        }

        const command = getShellCommandFromToolCall(toolRequest.toolCall);
        if (command && isRunCommand(command)) {
          // Get the tool name
          const toolCall = toolRequest.toolCall as Record<string, unknown>;
          const unwrapped = toolCall?.status === 'success'
            ? (toolCall.value as { name: string })
            : (toolCall as { name: string });

          return {
            command,
            toolId: toolRequest.id,
            toolName: unwrapped?.name || 'shell',
          };
        }
      }
    }
  }

  return null;
}
