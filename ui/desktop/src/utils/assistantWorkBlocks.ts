import type { Message } from '../api';

export interface WorkBlock {
  /** Indices of intermediate (collapsed) assistant messages */
  intermediateIndices: number[];
  /** All indices in the block range (assistant + user tool results) except final answer */
  allBlockIndices: Set<number>;
  /** Index of the "final answer" message shown normally, or -1 if none yet */
  finalIndex: number;
  /** Total tool calls across intermediates */
  toolCallCount: number;
  /** Whether this block is actively streaming */
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
function isRealUserMessage(message: Message, index: number, messages: Message[]): boolean {
  if (message.role !== 'user') return false;

  // Pure tool responses are never real user messages
  const hasOnlyToolResponses = message.content.every((c) => c.type === 'toolResponse');
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
 *
 * A "final answer" is the last assistant message in a run that has display
 * text but no tool requests, confirmations, or elicitations. During streaming,
 * if no such message exists yet, all messages stay collapsed in the work block
 * (finalIndex = -1).
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

  if (assistantRuns.length > 0) {
    // Log message roles/types around run boundaries for debugging
    for (const run of assistantRuns) {
      const roles = [];
      for (let i = run.start; i <= Math.min(run.end, run.start + 5); i++) {
        const m = messages[i];
        roles.push(`${i}:${m.role}(${m.content.map((c) => c.type).join(',')})`);
      }
      if (run.end - run.start > 5) roles.push('...');
    }
  }

  for (const run of assistantRuns) {
    // Collect all assistant message indices in this run
    const assistantIndices: number[] = [];
    for (let i = run.start; i <= run.end; i++) {
      if (messages[i].role === 'assistant') {
        assistantIndices.push(i);
      }
    }

    const isLastRunStreaming = isStreamingLast && run.end === messages.length - 1;

    // A single assistant message doesn't need grouping — unless it's streaming
    if (assistantIndices.length <= 1 && !isLastRunStreaming) {
      continue;
    }

    // Find the "final answer" — the message to show outside the collapsed block.
    // Strategy: prefer a clean text-only message, but accept a message with both
    // text and tool calls if no pure text message exists.
    // Always search regardless of streaming state.
    let finalAnswerIdx = -1;
    let textWithToolsIdx = -1;

    for (let i = assistantIndices.length - 1; i >= 0; i--) {
      const idx = assistantIndices[i];
      const msg = messages[idx];

      if (!hasDisplayText(msg)) continue;
      if (hasToolConfirmation(msg) || hasElicitation(msg)) continue;

      if (!hasToolRequests(msg)) {
        // Best case: pure text, no tool requests
        finalAnswerIdx = idx;
        break;
      } else if (textWithToolsIdx === -1) {
        // Fallback: has text AND tool requests — still a valid answer
        textWithToolsIdx = idx;
      }
    }

    // Use text-with-tools fallback if no pure text answer found
    if (finalAnswerIdx === -1 && textWithToolsIdx !== -1) {
      finalAnswerIdx = textWithToolsIdx;
    }

    // If no message with display text was found at all, keep everything collapsed
    // (finalIndex = -1). The WorkBlockIndicator will show "Worked on N steps".
    // For streaming runs, also keep finalIndex as -1 ("no final answer yet").

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
