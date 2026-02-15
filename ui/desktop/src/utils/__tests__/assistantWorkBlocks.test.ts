import { describe, it, expect } from 'vitest';
import { identifyWorkBlocks, WorkBlock } from '../assistantWorkBlocks';

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
    const result = identifyWorkBlocks(messages as any, false);
    expect(result.size).toBe(0);
  });

  it('returns empty map for user-assistant pair (no tool calls)', () => {
    const messages = [textMsg('user', 'Hello'), textMsg('assistant', 'Hi there!')];
    const result = identifyWorkBlocks(messages as any, false);
    expect(result.size).toBe(0);
  });

  it('creates a work block for assistant tool-call chain with final answer', () => {
    const messages = [
      textMsg('user', 'List files'),
      toolRequestMsg('shell'),
      toolResponseMsg('shell', 'file1.txt\nfile2.txt'),
      textMsg('assistant', 'Here are the files: file1.txt, file2.txt'),
    ];
    const result = identifyWorkBlocks(messages as any, false);

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
    const result = identifyWorkBlocks(messages as any, false);

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
    const result = identifyWorkBlocks(messages as any, true);

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
    const result = identifyWorkBlocks(messages as any, false);
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
    const result = identifyWorkBlocks(messages as any, false);

    // Block 1: indices 1, 2
    const block1 = result.get(1);
    expect(block1).toBeDefined();
    expect(block1!.finalIndex).toBe(3);

    // Block 2: indices 5, 6
    const block2 = result.get(5);
    expect(block2).toBeDefined();
    expect(block2!.finalIndex).toBe(7);

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
    const result = identifyWorkBlocks(messages as any, false);

    // All intermediates should be in ONE block
    const block1 = result.get(1);
    const block3 = result.get(3);
    expect(block1).toBeDefined();
    expect(block3).toBeDefined();
    expect(block1).toBe(block3); // Same block object
    expect(block1!.finalIndex).toBe(5);
  });

  it('handles final answer that has BOTH text and tool requests (two-tier)', () => {
    const messages = [
      textMsg('user', 'Help me'),
      toolRequestMsg('tool1'),
      toolResponseMsg('tool1', 'result1'),
      toolRequestAndTextMsg('tool2', 'Here is the answer with a tool call'),
    ];
    const result = identifyWorkBlocks(messages as any, false);

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
    const result = identifyWorkBlocks(messages as any, false);

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
    const result = identifyWorkBlocks(messages as any, false);

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
    const result = identifyWorkBlocks(messages as any, false);

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
    const streamingResult = identifyWorkBlocks(streamingMessages as any, true);
    const streamingBlock = streamingResult.get(1)!;
    expect(streamingBlock.isStreaming).toBe(true);
    expect(streamingBlock.finalIndex).toBe(-1);

    // Second call: completed (final answer arrived)
    const completedMessages = [
      ...streamingMessages,
      textMsg('assistant', 'Here are the files'),
    ];
    const completedResult = identifyWorkBlocks(completedMessages as any, false);
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
    const result = identifyWorkBlocks(messages as any, false);
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
    const result = identifyWorkBlocks(messages as any, false);
    const block = result.get(1)!;

    // intermediateIndices should be assistant messages only (1, 3)
    // NOT tool response messages (2, 4) which are user role
    for (const idx of block.intermediateIndices) {
      expect((messages as any)[idx].role).toBe('assistant');
    }
  });
});
