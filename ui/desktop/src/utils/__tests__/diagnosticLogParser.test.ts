import { describe, expect, it } from 'vitest';
import {
  Category,
  Zone,
  zoneOf,
  parseLogLines,
  parseSession,
  toMessages,
} from '../diagnosticLogParser';

// ── Test helpers ─────────────────────────────────────────────────────

// Long system prompt to avoid triggering title-generation detection (which checks sys.length < 200)
const DEFAULT_SYSTEM = 'You are goose, a general-purpose AI agent. You have access to tools for shell commands, file editing, and code analysis. Use these tools to help the user with their tasks. Always follow best practices and explain your reasoning. This system prompt is intentionally long enough to exceed the title-generation detection threshold of 200 characters.';

function makeInputLine(messages: Array<{ role: string; content: unknown }>, system = DEFAULT_SYSTEM): string {
  return JSON.stringify({
    model: 'claude-opus-4-6',
    input: { system, messages },
    tools: [],
    stream: true,
  });
}

function makeResponseLine(content: Array<{ type: string; [key: string]: unknown }>): string {
  return JSON.stringify({
    data: { id: 'resp-1', role: 'assistant', created: 1234567890, content },
  });
}

function makeUsageLine(input_tokens: number, output_tokens: number): string {
  return JSON.stringify({
    data: null,
    usage: { input_tokens, output_tokens, total_tokens: input_tokens + output_tokens },
  });
}

// ── Zone mapping ─────────────────────────────────────────────────────

describe('zoneOf', () => {
  it('maps main panel categories', () => {
    expect(zoneOf(Category.USER_INPUT)).toBe(Zone.MAIN_PANEL);
    expect(zoneOf(Category.ASSISTANT_TEXT)).toBe(Zone.MAIN_PANEL);
    expect(zoneOf(Category.STREAMING_CHUNK)).toBe(Zone.MAIN_PANEL);
  });

  it('maps work block categories', () => {
    expect(zoneOf(Category.TOOL_REQUEST)).toBe(Zone.WORK_BLOCK);
    expect(zoneOf(Category.TOOL_RESULT)).toBe(Zone.WORK_BLOCK);
    expect(zoneOf(Category.INTERMEDIATE_TEXT)).toBe(Zone.WORK_BLOCK);
  });

  it('maps reasoning category', () => {
    expect(zoneOf(Category.THINKING)).toBe(Zone.REASONING);
  });

  it('maps hidden categories', () => {
    expect(zoneOf(Category.SYSTEM_INFO)).toBe(Zone.HIDDEN);
    expect(zoneOf(Category.TITLE_GENERATION)).toBe(Zone.HIDDEN);
    expect(zoneOf(Category.USAGE_STATS)).toBe(Zone.HIDDEN);
  });
});

// ── parseLogLines ────────────────────────────────────────────────────

describe('parseLogLines', () => {
  it('parses a basic input + response + usage triplet', () => {
    const lines = [
      makeInputLine([
        { role: 'user', content: [{ type: 'text', text: 'Hello' }] },
      ]),
      makeResponseLine([{ type: 'text', text: 'Hi there!' }]),
      makeUsageLine(100, 20),
    ];

    const result = parseLogLines(lines, 'test.jsonl');
    expect(result.inputMsgs).toHaveLength(1);
    expect(result.isTitleGen).toBe(false);
    expect(result.items).toHaveLength(2);
    expect(result.items[0].category).toBe(Category.STREAMING_CHUNK);
    expect(result.items[0].text).toBe('Hi there!');
    expect(result.items[1].category).toBe(Category.USAGE_STATS);
  });

  it('detects title generation calls', () => {
    const lines = [
      makeInputLine(
        [{ role: 'user', content: 'Summarize the first few user messages' }],
        'Generate a short title'
      ),
      makeResponseLine([{ type: 'text', text: 'Code audit session' }]),
    ];

    const result = parseLogLines(lines, 'title.jsonl');
    expect(result.isTitleGen).toBe(true);
    expect(result.items[0].category).toBe(Category.TITLE_GENERATION);
    expect(result.items[0].text).toBe('Code audit session');
  });

  it('categorizes tool request responses', () => {
    const lines = [
      makeInputLine([{ role: 'user', content: [{ type: 'text', text: 'Fix the bug' }] }]),
      makeResponseLine([{
        type: 'toolRequest',
        id: 'tool-1',
        toolCall: { status: 'success', value: { name: 'developer__shell', arguments: { command: 'ls' } } },
      }]),
    ];

    const result = parseLogLines(lines, 'tools.jsonl');
    expect(result.items[0].category).toBe(Category.TOOL_REQUEST);
    expect(result.items[0].toolName).toBe('developer__shell');
  });

  it('categorizes thinking responses', () => {
    const lines = [
      makeInputLine([{ role: 'user', content: [{ type: 'text', text: 'Think about this' }] }]),
      makeResponseLine([{ type: 'thinking', thinking: 'Let me consider...' }]),
    ];

    const result = parseLogLines(lines, 'thinking.jsonl');
    expect(result.items[0].category).toBe(Category.THINKING);
  });

  it('handles empty lines gracefully', () => {
    const lines = ['', '  ', 'not-json', makeUsageLine(10, 5)];
    const result = parseLogLines(lines, 'messy.jsonl');
    expect(result.items).toHaveLength(1);
    expect(result.items[0].category).toBe(Category.USAGE_STATS);
  });

  it('handles streaming chunks in responses', () => {
    const lines = [
      makeInputLine([{ role: 'user', content: [{ type: 'text', text: 'Hi' }] }]),
      makeResponseLine([{ type: 'text', text: 'I' }]),
      makeResponseLine([{ type: 'text', text: "'ll help" }]),
      makeResponseLine([{ type: 'text', text: ' you.' }]),
      makeUsageLine(50, 10),
    ];

    const result = parseLogLines(lines, 'stream.jsonl');
    const chunks = result.items.filter((i) => i.category === Category.STREAMING_CHUNK);
    expect(chunks).toHaveLength(3);
    expect(chunks.map((c) => c.text).join('')).toBe("I'll help you.");
  });
});

// ── parseSession ─────────────────────────────────────────────────────

describe('parseSession', () => {
  function makeSessionFiles(): Map<string, string[]> {
    const files = new Map<string, string[]>();

    // File 0: user asks, assistant calls tool
    files.set('llm_request.0.jsonl', [
      makeInputLine([
        { role: 'user', content: [{ type: 'text', text: 'Analyze the code' }] },
      ]),
      makeResponseLine([{
        type: 'toolRequest',
        id: 'tool-1',
        toolCall: { status: 'success', value: { name: 'developer__shell', arguments: { command: 'ls' } } },
      }]),
      makeUsageLine(500, 30),
    ]);

    // File 1: full conversation with tool result and second tool call
    files.set('llm_request.1.jsonl', [
      makeInputLine([
        { role: 'user', content: [{ type: 'text', text: 'Analyze the code' }] },
        { role: 'assistant', content: [{ type: 'tool_use', id: 'tool-1', name: 'developer__shell' }] },
        { role: 'user', content: [{ type: 'tool_result', tool_use_id: 'tool-1', content: 'file1.ts\nfile2.ts' }] },
      ]),
      makeResponseLine([{
        type: 'toolRequest',
        id: 'tool-2',
        toolCall: { status: 'success', value: { name: 'developer__shell', arguments: { command: 'cat file1.ts' } } },
      }]),
      makeUsageLine(800, 40),
    ]);

    // File 2: full conversation with final streaming text
    files.set('llm_request.2.jsonl', [
      makeInputLine([
        { role: 'user', content: [{ type: 'text', text: 'Analyze the code' }] },
        { role: 'assistant', content: [{ type: 'tool_use', id: 'tool-1', name: 'developer__shell' }] },
        { role: 'user', content: [{ type: 'tool_result', tool_use_id: 'tool-1', content: 'file1.ts' }] },
        { role: 'assistant', content: [{ type: 'tool_use', id: 'tool-2', name: 'developer__shell' }] },
        { role: 'user', content: [{ type: 'tool_result', tool_use_id: 'tool-2', content: 'const x = 1;' }] },
      ]),
      makeResponseLine([{ type: 'text', text: 'The code looks ' }]),
      makeResponseLine([{ type: 'text', text: 'good.' }]),
      makeUsageLine(1200, 15),
    ]);

    // Title generation call
    files.set('llm_request.3.jsonl', [
      makeInputLine(
        [{ role: 'user', content: 'Summarize the first few user messages' }],
        'Generate a short title'
      ),
      makeResponseLine([{ type: 'text', text: 'Code analysis' }]),
    ]);

    return files;
  }

  it('reconstructs a session from multiple files', () => {
    const session = parseSession(makeSessionFiles());

    // Should use the most complete non-title file (file 2 with 5 messages)
    expect(session.conversationItems.length).toBe(5);
    expect(session.titleItems.length).toBe(1);
  });

  it('identifies user input in main panel', () => {
    const session = parseSession(makeSessionFiles());
    const userItems = session.conversationItems.filter((i) => i.category === Category.USER_INPUT);
    expect(userItems.length).toBe(1);
    expect(userItems[0].text).toContain('Analyze the code');
    expect(userItems[0].zone).toBe(Zone.MAIN_PANEL);
  });

  it('identifies tool results in work block', () => {
    const session = parseSession(makeSessionFiles());
    const toolResults = session.conversationItems.filter((i) => i.category === Category.TOOL_RESULT);
    expect(toolResults.length).toBeGreaterThan(0);
    expect(toolResults.every((i) => i.zone === Zone.WORK_BLOCK)).toBe(true);
  });

  it('identifies tool requests in work block', () => {
    const session = parseSession(makeSessionFiles());
    const toolReqs = session.conversationItems.filter((i) => i.category === Category.TOOL_REQUEST);
    expect(toolReqs.length).toBeGreaterThan(0);
    expect(toolReqs.every((i) => i.zone === Zone.WORK_BLOCK)).toBe(true);
  });

  it('accumulates streaming chunks in timeline', () => {
    const session = parseSession(makeSessionFiles());
    const streamingChunks = session.responseItems.filter((i) => i.category === Category.STREAMING_CHUNK);
    // File 2 has 2 text chunks, files 0-1 have tool responses, file 3 is title gen
    expect(streamingChunks.length).toBeGreaterThanOrEqual(2);

    // Timeline should accumulate them into a single assistant text
    const timelineMessages = session.timeline.filter(
      (e) => e.type === 'message' && e.item.category === Category.ASSISTANT_TEXT
    );
    // At least one accumulated message from streaming
    expect(timelineMessages.length).toBeGreaterThanOrEqual(1);
  });

  it('puts title generation in hidden zone', () => {
    const session = parseSession(makeSessionFiles());
    expect(session.titleItems.every((i) => i.zone === Zone.HIDDEN)).toBe(true);
  });

  it('computes zone counts correctly', () => {
    const session = parseSession(makeSessionFiles());
    const total = Object.values(session.zoneCounts).reduce((a, b) => a + b, 0);
    expect(total).toBeGreaterThan(0);
    expect(session.zoneCounts[Zone.MAIN_PANEL]).toBeGreaterThan(0);
    expect(session.zoneCounts[Zone.WORK_BLOCK]).toBeGreaterThan(0);
  });

  it('builds a coherent timeline', () => {
    const session = parseSession(makeSessionFiles());
    expect(session.timeline.length).toBeGreaterThan(0);

    // First timeline entry should be the user message
    const first = session.timeline[0];
    expect(first.type).toBe('message');
    if (first.type === 'message') {
      expect(first.item.category).toBe(Category.USER_INPUT);
    }

    // Should have at least one work block
    const workBlocks = session.timeline.filter((e) => e.type === 'work_block');
    expect(workBlocks.length).toBeGreaterThan(0);
  });
});

// ── toMessages ───────────────────────────────────────────────────────

describe('toMessages', () => {
  it('converts a session to Message[] for UI rendering', () => {
    const files = new Map<string, string[]>();
    files.set('test.jsonl', [
      makeInputLine([
        { role: 'user', content: [{ type: 'text', text: 'Hello' }] },
        { role: 'assistant', content: [{ type: 'text', text: 'Hi there!' }] },
      ]),
      makeUsageLine(50, 10),
    ]);

    const session = parseSession(files);
    const messages = toMessages(session);

    expect(messages.length).toBeGreaterThan(0);
    expect(messages[0].role).toBe('user');
    expect(messages[0].content[0].type).toBe('text');
  });

  it('filters hidden items from messages', () => {
    const files = new Map<string, string[]>();
    files.set('test.jsonl', [
      makeInputLine([
        { role: 'user', content: [{ type: 'text', text: '<info-msg>timestamp</info-msg>' }] },
        { role: 'user', content: [{ type: 'text', text: 'Real question' }] },
        { role: 'assistant', content: [{ type: 'text', text: 'Answer' }] },
      ]),
      makeUsageLine(50, 10),
    ]);

    const session = parseSession(files);
    const messages = toMessages(session);

    // Info-msg should be filtered out
    const userMsgs = messages.filter((m) => m.role === 'user');
    expect(userMsgs.length).toBe(1);
    expect(userMsgs[0].content[0].type).toBe('text');
  });
});

// ── Work block categorization ────────────────────────────────────────

describe('work block categorization', () => {
  it('categorizes a multi-turn tool conversation correctly', () => {
    const files = new Map<string, string[]>();
    files.set('test.jsonl', [
      makeInputLine([
        { role: 'user', content: [{ type: 'text', text: 'Fix the bug' }] },
        { role: 'assistant', content: [{ type: 'tool_use', id: 't1', name: 'shell' }] },
        { role: 'user', content: [{ type: 'tool_result', tool_use_id: 't1', content: 'error' }] },
        { role: 'assistant', content: [{ type: 'tool_use', id: 't2', name: 'shell' }] },
        { role: 'user', content: [{ type: 'tool_result', tool_use_id: 't2', content: 'fixed' }] },
        { role: 'assistant', content: [{ type: 'text', text: 'I fixed the bug.' }] },
      ]),
      makeUsageLine(500, 50),
    ]);

    const session = parseSession(files);

    // User message → main panel
    const user = session.conversationItems.find((i) => i.category === Category.USER_INPUT);
    expect(user).toBeDefined();
    expect(user!.zone).toBe(Zone.MAIN_PANEL);

    // Final answer → main panel (last assistant with text, no tools)
    const answer = session.conversationItems.find((i) => i.category === Category.ASSISTANT_TEXT);
    expect(answer).toBeDefined();
    expect(answer!.zone).toBe(Zone.MAIN_PANEL);
    expect(answer!.summary).toContain('fixed the bug');

    // Tool requests and results → work block
    const tools = session.conversationItems.filter((i) => i.zone === Zone.WORK_BLOCK);
    expect(tools.length).toBe(4); // 2 tool requests + 2 tool results
  });

  it('handles single assistant message without work block', () => {
    const files = new Map<string, string[]>();
    files.set('test.jsonl', [
      makeInputLine([
        { role: 'user', content: [{ type: 'text', text: 'Hi' }] },
        { role: 'assistant', content: [{ type: 'text', text: 'Hello!' }] },
      ]),
      makeUsageLine(20, 5),
    ]);

    const session = parseSession(files);
    const workBlockItems = session.conversationItems.filter((i) => i.zone === Zone.WORK_BLOCK);
    expect(workBlockItems.length).toBe(0);
  });
});
