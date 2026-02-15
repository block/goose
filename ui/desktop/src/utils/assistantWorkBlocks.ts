/**
 * Groups consecutive assistant messages into "work blocks" for hidden mode.
 *
 * In ChatGPT, all intermediate reasoning / tool work is collapsed into a
 * single "Thought for X seconds" toggle. Goose emits multiple assistant
 * messages per turn (narration → tool calls → narration → tool calls → final answer).
 *
 * This utility identifies those runs so the UI can collapse them into one
 * visual block, showing only the final answer normally.
 *
 * A "work block" is a consecutive run of assistant messages between real user
 * messages. User messages that are tool responses or summarized tool results
 * (injected by the system between assistant messages) are treated as part of
 * the work block, not as boundaries.
 */

import { Message } from '../api';

export interface WorkBlock {
  /** Indices of intermediate assistant messages to collapse */
  intermediateIndices: number[];
  /** ALL message indices in this block (assistant + user tool results) to hide */
  allBlockIndices: Set<number>;
  /** Index of the final answer message (shown normally), or -1 if streaming */
  finalIndex: number;
  /** Total tool calls across all intermediate messages */
  toolCallCount: number;
  /** Whether the block is still streaming (final answer not yet determined) */
  isStreaming: boolean;
}

function hasDisplayText(message: Message): boolean {
  return message.content.some(
    (c) => c.type === 'text' && typeof c.text === 'string' && c.text.trim().length > 0
  );
}

function hasToolRequests(message: Message): boolean {
  return message.content.some((c) => c.type === 'toolRequest');
}

function countToolRequests(message: Message): number {
  return message.content.filter((c) => c.type === 'toolRequest').length;
}

function hasToolConfirmation(message: Message): boolean {
  return message.content.some((c) => c.type === 'toolConfirmationRequest');
}

function hasElicitation(message: Message): boolean {
  return message.content.some(
    (c) =>
      c.type === 'actionRequired' &&
      'data' in c &&
      (c.data as Record<string, unknown>)?.actionType === 'elicitation'
  );
}

/**
 * Determines if a user message is a "real" user input vs a system-injected
 * tool result. System-injected messages include:
 * - Messages with only toolResponse content
 * - Messages that follow an assistant toolRequest (summarized tool results)
 *
 * Real user messages are the initial request and any follow-up user inputs
 * that don't follow a tool call cycle.
 */
function isRealUserMessage(
  message: Message,
  index: number,
  messages: Message[]
): boolean {
  if (message.role !== 'user') return false;

  // Pure tool responses are never real user messages
  const hasOnlyToolResponses = message.content.every(
    (c) => c.type === 'toolResponse'
  );
  if (hasOnlyToolResponses) return false;

  // If the previous assistant message had tool requests, this user message
  // is likely a summarized tool result (the system injects these)
  for (let i = index - 1; i >= 0; i--) {
    const prev = messages[i];
    if (prev.role === 'assistant') {
      return !hasToolRequests(prev);
    }
    // Skip other user messages (tool responses) to find the preceding assistant
    if (prev.role === 'user') {
      const prevIsToolResp = prev.content.every((c) => c.type === 'toolResponse');
      if (prevIsToolResp) continue;
      // Another real user message before us — we're also real
      return true;
    }
  }

  // First message in the conversation — it's real
  return true;
}

/**
 * Identifies work blocks in the message list.
 *
 * Returns a Map from message index → WorkBlock for each intermediate
 * message that should be collapsed. Messages not in the map are rendered
 * normally.
 */
export function identifyWorkBlocks(
  messages: Message[],
  isStreamingLast: boolean
): Map<number, WorkBlock> {
  const result = new Map<number, WorkBlock>();

  // Find runs of consecutive assistant messages (with transparent user messages)
  let blockStart = -1;
  const assistantRuns: Array<{ start: number; end: number }> = [];

  for (let i = 0; i < messages.length; i++) {
    const msg = messages[i];
    const isAssistant = msg.role === 'assistant';

    if (isAssistant && blockStart === -1) {
      blockStart = i;
    } else if (!isAssistant && blockStart !== -1) {
      // Only break the run on REAL user messages, not tool results
      if (isRealUserMessage(msg, i, messages)) {
        assistantRuns.push({ start: blockStart, end: i - 1 });
        blockStart = -1;
      }
    }
  }
  // Close final run
  if (blockStart !== -1) {
    assistantRuns.push({ start: blockStart, end: messages.length - 1 });
  }

  for (const run of assistantRuns) {
    // Collect all assistant message indices in this run
    const assistantIndices: number[] = [];
    for (let i = run.start; i <= run.end; i++) {
      if (messages[i].role === 'assistant') {
        assistantIndices.push(i);
      }
    }

    // A single assistant message doesn't need grouping
    if (assistantIndices.length <= 1) continue;

    // Find the last assistant message with display text — that's the "final answer"
    // Skip messages that also have tool calls (those are intermediate narration+tool combos)
    // Also skip if it has pending confirmations or elicitations
    let finalAnswerIdx = -1;
    const isLastRunStreaming = isStreamingLast && run.end === messages.length - 1;

    if (!isLastRunStreaming) {
      for (let i = assistantIndices.length - 1; i >= 0; i--) {
        const idx = assistantIndices[i];
        const msg = messages[idx];
        if (
          hasDisplayText(msg) &&
          !hasToolRequests(msg) &&
          !hasToolConfirmation(msg) &&
          !hasElicitation(msg)
        ) {
          finalAnswerIdx = idx;
          break;
        }
      }
    }

    // If no final answer found and not streaming, the last message IS the final answer
    if (finalAnswerIdx === -1 && !isLastRunStreaming) {
      finalAnswerIdx = assistantIndices[assistantIndices.length - 1];
    }

    // Count total tool calls across intermediate messages
    let totalToolCalls = 0;
    const intermediateIndices: number[] = [];

    for (const idx of assistantIndices) {
      if (idx === finalAnswerIdx) continue;
      intermediateIndices.push(idx);
      totalToolCalls += countToolRequests(messages[idx]);
    }

    if (intermediateIndices.length === 0) continue;

    // Collect ALL indices in the block range (assistant + user) except the final answer
    const allBlockIndices = new Set<number>();
    for (let i = run.start; i <= run.end; i++) {
      if (i !== finalAnswerIdx) {
        allBlockIndices.add(i);
      }
    }

    const block: WorkBlock = {
      intermediateIndices,
      allBlockIndices,
      finalIndex: finalAnswerIdx,
      toolCallCount: totalToolCalls,
      isStreaming: isLastRunStreaming,
    };

    // Map EVERY index in the block (assistant AND user) to this block
    for (const idx of allBlockIndices) {
      result.set(idx, block);
    }
  }

  return result;
}
