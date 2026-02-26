import { describe, expect, it } from 'vitest';
import type { Message } from '@/api';
import type { WorkBlock } from '../assistantWorkBlocks';
import { identifyWorkBlocks } from '../assistantWorkBlocks';

// Helper to create a minimal Message-like object
function msg(
  role: 'user' | 'assistant',
  content: Array<{ type: string; [key: string]: unknown }>,
  id?: string
) {
  return {
    role,
    content,
    id: id ?? `msg-${Math.random().toString(36).slice(2, 8)}`,
    created: Date.now() / 1000,
  };
}

function textMsg(role: 'user' | 'assistant', text: string, id?: string) {
  return msg(role, [{ type: 'text', text }], id);
}

function toolRequestMsg(toolName: string, id?: string) {
  return msg(
    'assistant',
    [
      {
        type: 'toolRequest',
        id: `tool-${toolName}`,
        name: toolName,
        input: '{}',
      },
    ],
    id
  );
}

function toolResponseMsg(toolName: string, output: string, id?: string) {
  return msg(
    'user',
    [
      {
        type: 'toolResponse',
        id: `tool-${toolName}`,
        name: toolName,
        output,
      },
    ],
    id
  );
}

function toolRequestAndTextMsg(toolName: string, text: string, id?: string) {
  return msg(
    'assistant',
    [
      { type: 'text', text },
      {
        type: 'toolRequest',
        id: `tool-${toolName}`,
        name: toolName,
        input: '{}',
      },
    ],
    id
  );
}

describe('identifyWorkBlocks', () => {
  it('returns empty map for empty messages', () => {
    const result = identifyWorkBlocks([], false);
    expect(result.size).toBe(0);
  });

  it('returns empty map for a single user message', () => {
    const messages = [textMsg('user', 'Hello')];
    const result = identifyWorkBlocks(messages as unknown as Message[], false);
    expect(result.size).toBe(0);
  });

  it('returns empty map for user-assistant pair (no tool calls)', () => {
    const messages = [textMsg('user', 'Hello'), textMsg('assistant', 'Hi there!')];
    const result = identifyWorkBlocks(messages as unknown as Message[], false);
    expect(result.size).toBe(0);
  });

  it('creates a work block for assistant tool-call chain with final answer', () => {
    const messages = [
      textMsg('user', 'List files'),
      toolRequestMsg('shell'),
      toolResponseMsg('shell', 'file1.txt\nfile2.txt'),
      textMsg('assistant', 'Here are the files: file1.txt, file2.txt'),
    ];
    const result = identifyWorkBlocks(messages as unknown as Message[], false);

    // Indices 1 (tool request) and 2 (tool response) should be in the block
    expect(result.has(1)).toBe(true);
    expect(result.has(2)).toBe(true);

    // Index 3 (final answer) should NOT be in the block
    expect(result.has(3)).toBe(false);

    // Index 0 (user message) should NOT be in the block
    expect(result.has(0)).toBe(false);

    const block = result.get(1)!;
    expect(block.finalIndex).toBe(3);
    expect(block.isStreaming).toBe(false);
    expect(block.toolCallCount).toBeGreaterThanOrEqual(1);
  });

  it('handles multiple tool calls before final answer', () => {
    const messages = [
      textMsg('user', 'Find and read the config'),
      toolRequestMsg('shell_find'),
      toolResponseMsg('shell_find', 'config.yaml'),
      toolRequestMsg('shell_read'),
      toolResponseMsg('shell_read', 'key: value'),
      textMsg('assistant', 'The config contains key: value'),
    ];
    const result = identifyWorkBlocks(messages as unknown as Message[], false);

    // All intermediate messages should be in the block
    expect(result.has(1)).toBe(true);
    expect(result.has(2)).toBe(true);
    expect(result.has(3)).toBe(true);
    expect(result.has(4)).toBe(true);

    // Final answer should NOT be in the block
    expect(result.has(5)).toBe(false);

    const block = result.get(1)!;
    expect(block.finalIndex).toBe(5);
    expect(block.toolCallCount).toBe(2);
  });

  it('keeps all messages in block during streaming (finalIndex = -1)', () => {
    const messages = [
      textMsg('user', 'List files'),
      toolRequestMsg('shell'),
      toolResponseMsg('shell', 'file1.txt'),
      toolRequestMsg('shell2'),
    ];
    const result = identifyWorkBlocks(messages as unknown as Message[], true);

    // During streaming, all intermediate messages should be in the block
    expect(result.has(1)).toBe(true);
    expect(result.has(2)).toBe(true);
    expect(result.has(3)).toBe(true);

    const block = result.get(1)!;
    expect(block.finalIndex).toBe(-1);
    expect(block.isStreaming).toBe(true);
  });

  it('does NOT create a block for a single assistant message without tool calls', () => {
    const messages = [textMsg('user', 'Hello'), textMsg('assistant', 'Hi!')];
    const result = identifyWorkBlocks(messages as unknown as Message[], false);
    expect(result.size).toBe(0);
  });

  it('creates separate blocks for two runs separated by real user message', () => {
    const messages = [
      textMsg('user', 'First task'),
      toolRequestMsg('tool1'),
      toolResponseMsg('tool1', 'result1'),
      textMsg('assistant', 'Done with first task'),
      textMsg('user', 'Second task'), // Real user message splits the runs
      toolRequestMsg('tool2'),
      toolResponseMsg('tool2', 'result2'),
      textMsg('assistant', 'Done with second task'),
    ];
    const result = identifyWorkBlocks(messages as unknown as Message[], false);

    // Block 1: indices 1, 2
    const block1 = result.get(1);
    expect(block1).toBeDefined();
    expect(block1?.finalIndex).toBe(3);

    // Block 2: indices 5, 6
    const block2 = result.get(5);
    expect(block2).toBeDefined();
    expect(block2?.finalIndex).toBe(7);

    // They should be different blocks
    expect(block1).not.toBe(block2);
  });

  it('does NOT treat tool response user messages as run boundaries', () => {
    const messages = [
      textMsg('user', 'Do something'),
      toolRequestMsg('tool1'),
      toolResponseMsg('tool1', 'result1'), // This is a "fake" user message
      toolRequestMsg('tool2'),
      toolResponseMsg('tool2', 'result2'), // This too
      textMsg('assistant', 'All done'),
    ];
    const result = identifyWorkBlocks(messages as unknown as Message[], false);

    // All intermediates should be in ONE block
    const block1 = result.get(1);
    const block3 = result.get(3);
    expect(block1).toBeDefined();
    expect(block3).toBeDefined();
    expect(block1).toBe(block3); // Same block object
    expect(block1?.finalIndex).toBe(5);
  });

  it('handles final answer that has BOTH text and tool requests (two-tier)', () => {
    const messages = [
      textMsg('user', 'Help me'),
      toolRequestMsg('tool1'),
      toolResponseMsg('tool1', 'result1'),
      toolRequestAndTextMsg('tool2', 'Here is the answer with a tool call'),
    ];
    const result = identifyWorkBlocks(messages as unknown as Message[], false);

    // Index 3 should be the final answer (has text + tool)
    expect(result.has(3)).toBe(false); // Final answer is excluded from block
    const block = result.get(1)!;
    expect(block.finalIndex).toBe(3);
  });

  it('prefers pure text over text+tool for final answer', () => {
    const messages = [
      textMsg('user', 'Help me'),
      toolRequestMsg('tool1'),
      toolResponseMsg('tool1', 'result1'),
      toolRequestAndTextMsg('tool2', 'Intermediate text with tool'),
      toolResponseMsg('tool2', 'result2'),
      textMsg('assistant', 'Pure text final answer'),
    ];
    const result = identifyWorkBlocks(messages as unknown as Message[], false);

    const block = result.get(1)!;
    // Should prefer index 5 (pure text) over index 3 (text+tool)
    expect(block.finalIndex).toBe(5);
    expect(result.has(5)).toBe(false); // Final answer excluded
    expect(result.has(3)).toBe(true); // text+tool is intermediate
  });

  it('returns finalIndex=-1 when no message has display text (completed run)', () => {
    const messages = [
      textMsg('user', 'Do something'),
      toolRequestMsg('tool1'),
      toolResponseMsg('tool1', 'result1'),
      toolRequestMsg('tool2'),
    ];
    const result = identifyWorkBlocks(messages as unknown as Message[], false);

    // No assistant message has display text → finalIndex should be -1
    const block = result.get(1)!;
    expect(block.finalIndex).toBe(-1);

    // ALL assistant messages should be in the block (nothing leaks out)
    expect(result.has(1)).toBe(true);
    expect(result.has(2)).toBe(true);
    expect(result.has(3)).toBe(true);
  });

  it('does not produce duplicate WorkBlockIndicators for same run', () => {
    const messages = [
      textMsg('user', 'List files'),
      toolRequestMsg('shell'),
      toolResponseMsg('shell', 'file1.txt'),
      toolRequestMsg('read'),
      toolResponseMsg('read', 'content'),
      textMsg('assistant', 'Here is the file content'),
    ];
    const result = identifyWorkBlocks(messages as unknown as Message[], false);

    // Count unique blocks
    const uniqueBlocks = new Set<WorkBlock>();
    result.forEach((block) => uniqueBlocks.add(block));
    expect(uniqueBlocks.size).toBe(1);
  });

  it('handles streaming transition: streaming then completed', () => {
    // First call: streaming
    const streamingMessages = [
      textMsg('user', 'List files'),
      toolRequestMsg('shell'),
      toolResponseMsg('shell', 'file1.txt'),
    ];
    const streamingResult = identifyWorkBlocks(streamingMessages as unknown as Message[], true);
    const streamingBlock = streamingResult.get(1)!;
    expect(streamingBlock.isStreaming).toBe(true);
    expect(streamingBlock.finalIndex).toBe(-1);

    // Second call: completed (final answer arrived)
    const completedMessages = [...streamingMessages, textMsg('assistant', 'Here are the files')];
    const completedResult = identifyWorkBlocks(completedMessages as unknown as Message[], false);
    const completedBlock = completedResult.get(1)!;
    expect(completedBlock.isStreaming).toBe(false);
    expect(completedBlock.finalIndex).toBe(3);
    expect(completedResult.has(3)).toBe(false); // Final answer excluded
  });

  it('allBlockIndices includes all messages except final answer', () => {
    const messages = [
      textMsg('user', 'Do it'),
      toolRequestMsg('tool1'),
      toolResponseMsg('tool1', 'r1'),
      toolRequestMsg('tool2'),
      toolResponseMsg('tool2', 'r2'),
      textMsg('assistant', 'Done'),
    ];
    const result = identifyWorkBlocks(messages as unknown as Message[], false);
    const block = result.get(1)!;

    // allBlockIndices should contain 1, 2, 3, 4 but NOT 5 (final) or 0 (user)
    expect(block.allBlockIndices.has(1)).toBe(true);
    expect(block.allBlockIndices.has(2)).toBe(true);
    expect(block.allBlockIndices.has(3)).toBe(true);
    expect(block.allBlockIndices.has(4)).toBe(true);
    expect(block.allBlockIndices.has(5)).toBe(false);
    expect(block.allBlockIndices.has(0)).toBe(false);
  });

  it('intermediateIndices contains only assistant messages', () => {
    const messages = [
      textMsg('user', 'Do it'),
      toolRequestMsg('tool1'),
      toolResponseMsg('tool1', 'r1'),
      toolRequestMsg('tool2'),
      toolResponseMsg('tool2', 'r2'),
      textMsg('assistant', 'Done'),
    ];
    const result = identifyWorkBlocks(messages as unknown as Message[], false);
    const block = result.get(1)!;

    // intermediateIndices should be assistant messages only (1, 3)
    // NOT tool response messages (2, 4) which are user role
    for (const idx of block.intermediateIndices) {
      expect((messages as unknown as Message[])[idx].role).toBe('assistant');
    }
  });

  // --- Non-regression: transient tool call flash (c97c7518) ---
  // During streaming, an assistant message transitions from text-only to text+toolRequests.
  // Before workBlocks recognizes it as a block, the tool calls could flash in the main chat.
  // These tests verify the workBlocks computation under those transitional states.

  it('NR: single streaming assistant with only text does NOT create a work block', () => {
    // Phase 1 of transient flash: message has text only (no tool calls yet)
    const messages = [textMsg('user', 'List files'), textMsg('assistant', 'Let me check...')];
    const result = identifyWorkBlocks(messages as unknown as Message[], true);

    // Single text-only assistant → no block (text renders normally in chat)
    expect(result.size).toBe(0);
  });

  it('NR: single streaming assistant with text+toolRequests is NOT in workBlocks (rendering suppresses)', () => {
    // Phase 2 of transient flash: tool requests arrive on the same message.
    // A single assistant message with text+tools becomes the "final answer" of a
    // 1-message run → zero intermediates → no block created.
    // Tool call suppression is handled by ProgressiveMessageList (suppressToolCalls prop)
    // not by identifyWorkBlocks.
    const messages = [
      textMsg('user', 'List files'),
      toolRequestAndTextMsg('shell', 'Let me check...'),
    ];
    const result = identifyWorkBlocks(messages as unknown as Message[], true);

    // Single text+tools assistant = final answer with 0 intermediates → no block
    expect(result.size).toBe(0);
    // The rendering layer (ProgressiveMessageList) detects this case and
    // passes suppressToolCalls=true to GooseMessage to prevent the flash.
  });

  it('NR: streaming assistant tool-only msg after text msg creates block (rendering layer tested separately)', () => {
    // When a second assistant message with tool requests arrives in the same run,
    // there ARE intermediates → a block is created.
    // The first message (text only) becomes final answer.
    const messages = [
      textMsg('user', 'Do X'),
      textMsg('assistant', 'Working...'), // first response, text only
      textMsg('user', 'Do Y'), // real second user message — splits runs
      toolRequestAndTextMsg('read', 'Reading file'), // streaming, text+tools in NEW run
    ];
    const result = identifyWorkBlocks(messages as unknown as Message[], true);

    // Run [3..3]: single assistant with text+tools → same as above: no block
    // (single message is the final answer, zero intermediates)
    // Run [1..1]: single text-only assistant → no block (no tool calls)
    expect(result.size).toBe(0);
    // Tool call suppression at rendering layer prevents the flash
  });

  // --- Non-regression: streaming text leak (keep final answer suppressed) ---
  // During streaming with multiple assistant messages, the last message often has
  // text content before tool calls arrive. Without suppression, this gets selected
  // as the "final answer" and renders outside the work block — causing content to
  // flash in and out as tool requests arrive.

  it('NR: multi-message streaming shows pure-text final answer for progressive rendering', () => {
    // Scenario: assistant made 2 tool calls, now streaming a 3rd message with text.
    // The pure-text message should be shown as the final answer so the user can
    // read the streamed text progressively while tool calls stay collapsed above.
    const messages = [
      textMsg('user', 'Analyze the code'),
      toolRequestMsg('shell'),
      toolResponseMsg('shell', 'file list'),
      toolRequestMsg('read_file'),
      toolResponseMsg('read_file', 'file contents'),
      textMsg('assistant', 'I see the issue, let me fix it...'), // streaming text — shown as final answer
    ];
    const result = identifyWorkBlocks(messages as unknown as Message[], true);

    // Tool call messages should be in the block
    expect(result.has(1)).toBe(true);
    expect(result.has(2)).toBe(true);
    expect(result.has(3)).toBe(true);
    expect(result.has(4)).toBe(true);
    // The pure-text message at index 5 is the final answer — NOT in the block
    expect(result.has(5)).toBe(false);

    const block = result.get(1)!;
    expect(block.finalIndex).toBe(5);
    expect(block.isStreaming).toBe(true);
    // The text-only message at index 5 is the final answer, not intermediate
    expect(block.intermediateIndices).not.toContain(5);
  });

  it('NR: multi-message streaming with text+tools keeps everything collapsed', () => {
    // Scenario: assistant streamed text, then tool request appeared on same message
    const messages = [
      textMsg('user', 'Fix the bug'),
      toolRequestMsg('shell'),
      toolResponseMsg('shell', 'error output'),
      toolRequestAndTextMsg('read_file', 'Let me read the file...'),
    ];
    const result = identifyWorkBlocks(messages as unknown as Message[], true);

    // All messages in block, no final answer
    expect(result.has(1)).toBe(true);
    expect(result.has(2)).toBe(true);
    expect(result.has(3)).toBe(true);

    const block = result.get(1)!;
    expect(block.finalIndex).toBe(-1);
    expect(block.isStreaming).toBe(true);
  });

  // --- Non-regression: dual WorkBlockIndicator (74b5de97) ---
  // The pending indicator was showing alongside a streaming work block because
  // tool-response user messages at the end of the array made lastMessage.role === 'user'.

  it('NR: work block exists during streaming when last message is tool response (user role)', () => {
    // The scenario: assistant sends tool request, user tool response arrives,
    // lastMessage.role is 'user' but a streaming work block should exist
    const messages = [
      textMsg('user', 'Analyze this'),
      toolRequestMsg('shell'),
      toolResponseMsg('shell', 'output'),
    ];
    const result = identifyWorkBlocks(messages as unknown as Message[], true);

    // Despite lastMessage being user role, a streaming work block should exist
    expect(result.size).toBeGreaterThan(0);
    const block = result.get(1)!;
    expect(block.isStreaming).toBe(true);
    // This is used by ProgressiveMessageList to suppress the pending indicator
  });

  it('NR: work block covers tool response messages even though they have user role', () => {
    const messages = [
      textMsg('user', 'Do something'),
      toolRequestMsg('tool1'),
      toolResponseMsg('tool1', 'result1'),
      toolRequestMsg('tool2'),
      toolResponseMsg('tool2', 'result2'),
    ];
    const result = identifyWorkBlocks(messages as unknown as Message[], true);

    // Tool response messages (indices 2, 4) should be in the block
    expect(result.has(2)).toBe(true);
    expect(result.has(4)).toBe(true);
    // The block should be streaming with finalIndex=-1
    const block = result.get(1)!;
    expect(block.isStreaming).toBe(true);
    expect(block.finalIndex).toBe(-1);
  });

  it('NR: two-tier final answer prefers pure text over text+tools', () => {
    // Non-regression for fe42b373
    const messages = [
      textMsg('user', 'Do things'),
      toolRequestAndTextMsg('tool1', 'Intermediate result'),
      toolResponseMsg('tool1', 'done'),
      textMsg('assistant', 'Here is the final answer'),
    ];
    const result = identifyWorkBlocks(messages as unknown as Message[], false);

    const block = result.get(1)!;
    // Pure text message (index 3) should be the final answer, not text+tools (index 1)
    expect(block.finalIndex).toBe(3);
    expect(result.has(3)).toBe(false); // Final answer excluded from block
    expect(result.has(1)).toBe(true); // text+tools message stays in block
  });

  it('NR: final answer with text+tools when no pure text exists', () => {
    // Non-regression for fe42b373
    const messages = [
      textMsg('user', 'Do things'),
      toolRequestMsg('tool1'),
      toolResponseMsg('tool1', 'done'),
      toolRequestAndTextMsg('tool2', 'Here is the answer with a tool call'),
    ];
    const result = identifyWorkBlocks(messages as unknown as Message[], false);

    const block = result.get(1)!;
    // text+tools message (index 3) should be final answer when no pure text exists
    expect(block.finalIndex).toBe(3);
    expect(result.has(3)).toBe(false); // Final answer excluded from block
  });

  it('NR: hidden user messages (userVisible=false) do not break work blocks', () => {
    // When compaction injects tool result summaries as user messages with
    // userVisible=false, they should not split assistant work blocks.
    const hiddenUserMsg = {
      ...textMsg('user', 'A shell command searched the codebase...'),
      metadata: { userVisible: false, agentVisible: true },
    };
    const messages = [
      textMsg('user', 'Fix the bug'),       // 0: real user message
      toolRequestMsg('tool1'),               // 1: assistant tool call
      toolResponseMsg('tool1', 'result'),    // 2: tool response
      hiddenUserMsg,                         // 3: hidden compacted summary
      textMsg('assistant', 'Found it'),      // 4: assistant text
      toolRequestMsg('tool2'),               // 5: assistant tool call
      toolResponseMsg('tool2', 'done'),      // 6: tool response
      textMsg('assistant', 'Here is the fix'), // 7: final answer
    ];
    const result = identifyWorkBlocks(messages as unknown as Message[], false);

    // All intermediate messages should be in ONE block (hidden user msg doesn't split)
    expect(result.has(1)).toBe(true);
    expect(result.has(4)).toBe(true);
    expect(result.has(5)).toBe(true);

    const block = result.get(1)!;
    // Final answer should be the last text message
    expect(block.finalIndex).toBe(7);
    expect(result.has(7)).toBe(false);
    // Hidden user message should be inside the block
    expect(block.allBlockIndices.has(3)).toBe(true);
  });
});
