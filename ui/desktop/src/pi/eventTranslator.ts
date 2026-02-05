/**
 * Translates Pi agent events to Goose-compatible message format.
 *
 * This allows the existing UI components to work with Pi as the backend,
 * maintaining a consistent user experience.
 */

import type { Message, MessageContent, Role } from '../api';

// Pi event types (from @mariozechner/pi-agent-core)
export interface PiAgentEvent {
  type: string;
  message?: PiAgentMessage;
  [key: string]: unknown;
}

export interface PiMessageContent {
  type: 'text' | 'thinking' | 'tool_call' | 'tool_result' | 'image';
  text?: string;
  thinking?: string;
  toolCallId?: string;
  toolName?: string;
  args?: Record<string, unknown>;
  result?: unknown;
  isError?: boolean;
}

export interface PiAgentMessage {
  id: string;
  role: 'user' | 'assistant' | 'tool';
  content: PiMessageContent[];
  createdAt?: string;
}

// Re-export Goose Message type for convenience
export type GooseMessage = Message;

/**
 * Translates a Pi message to Goose message format.
 */
export function translatePiMessage(piMessage: PiAgentMessage): GooseMessage {
  const gooseContent: MessageContent[] = [];

  for (const content of piMessage.content) {
    switch (content.type) {
      case 'text':
        gooseContent.push({
          type: 'text',
          text: content.text || '',
        });
        break;

      case 'thinking':
        gooseContent.push({
          type: 'thinking',
          thinking: content.thinking || content.text || '',
          signature: '', // Pi doesn't use Anthropic-style thinking signatures
        });
        break;

      case 'tool_call':
        gooseContent.push({
          type: 'toolRequest',
          id: content.toolCallId || generateId(),
          toolCall: {
            name: content.toolName || 'unknown',
            arguments: content.args || {},
          },
        });
        break;

      case 'tool_result':
        gooseContent.push({
          type: 'toolResponse',
          id: content.toolCallId || generateId(),
          toolResult: {
            result: content.result,
            isError: content.isError || false,
          },
        });
        break;
    }
  }

  const role: Role = piMessage.role === 'tool' ? 'assistant' : piMessage.role;
  const created = piMessage.createdAt
    ? Math.floor(new Date(piMessage.createdAt).getTime() / 1000)
    : Math.floor(Date.now() / 1000);

  return {
    id: piMessage.id,
    role,
    created,
    content: gooseContent,
    metadata: {
      userVisible: true,
      agentVisible: true,
    },
  };
}

function generateId(): string {
  return Math.random().toString(36).substring(2, 10);
}

/**
 * State machine for accumulating Pi events into Goose messages.
 */
export class PiEventAccumulator {
  private currentMessage: PiAgentMessage | null = null;
  private messages: GooseMessage[] = [];
  private isStreaming = false;

  /**
   * Process a Pi event and return any completed Goose messages.
   */
  processEvent(event: PiAgentEvent): {
    message?: GooseMessage;
    isComplete: boolean;
    isStreaming: boolean;
  } {
    switch (event.type) {
      case 'agent_start':
        this.isStreaming = true;
        this.messages = [];
        return { isComplete: false, isStreaming: true };

      case 'agent_end':
        this.isStreaming = false;
        return { isComplete: true, isStreaming: false };

      case 'message_start': {
        const msg = event.message as PiAgentMessage | undefined;
        if (!msg) return { isComplete: false, isStreaming: true };
        this.currentMessage = msg;
        return {
          message: translatePiMessage(msg),
          isComplete: false,
          isStreaming: true,
        };
      }

      case 'message_update': {
        const msg = event.message as PiAgentMessage | undefined;
        if (!msg) return { isComplete: false, isStreaming: true };
        this.currentMessage = msg;
        return {
          message: translatePiMessage(msg),
          isComplete: false,
          isStreaming: true,
        };
      }

      case 'message_end': {
        const msg = event.message as PiAgentMessage | undefined;
        if (!msg) return { isComplete: false, isStreaming: true };
        this.currentMessage = null;
        const gooseMsg = translatePiMessage(msg);
        this.messages.push(gooseMsg);
        return {
          message: gooseMsg,
          isComplete: false,
          isStreaming: true,
        };
      }

      case 'tool_execution_start': {
        // Tool execution events are already part of the message content
        // We can emit a partial update here if needed
        return { isComplete: false, isStreaming: true };
      }

      case 'tool_execution_end': {
        // Tool result will be included in the next message
        return { isComplete: false, isStreaming: true };
      }

      default:
        // Unknown event type, ignore
        return { isComplete: false, isStreaming: this.isStreaming };
    }
  }

  /**
   * Get all accumulated messages.
   */
  getMessages(): GooseMessage[] {
    return this.messages;
  }

  /**
   * Get the current streaming message (if any).
   */
  getCurrentMessage(): GooseMessage | null {
    return this.currentMessage ? translatePiMessage(this.currentMessage) : null;
  }

  /**
   * Reset the accumulator.
   */
  reset(): void {
    this.currentMessage = null;
    this.messages = [];
    this.isStreaming = false;
  }
}

/**
 * Create SSE-compatible event data from a Pi event.
 * This matches the format goosed uses for /reply SSE stream.
 */
export function piEventToSSE(event: PiAgentEvent): string | null {
  const result = new PiEventAccumulator().processEvent(event);

  if (result.message) {
    return JSON.stringify({
      type: 'message',
      data: result.message,
    });
  }

  if (result.isComplete) {
    return JSON.stringify({
      type: 'done',
    });
  }

  return null;
}
