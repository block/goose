import { describe, expect, it } from 'vitest';
import type { Message } from '@/api';
import {
  buildToolResponseMap,
  extractActivityEntries,
  type ToolActivityEntry,
} from '../ReasoningDetailPanel';

// ── Helpers ──────────────────────────────────────────────────────────

function makeMsg(
  role: string,
  content: Array<Record<string, unknown>>,
  id?: string
): Message {
  return {
    role,
    content: content as Message['content'],
    id: id || `msg-${Math.random().toString(36).slice(2, 8)}`,
    created: Date.now() / 1000,
    metadata: { agentVisible: true, userVisible: true },
  };
}

function makeToolRequest(
  name: string,
  args: Record<string, unknown>,
  requestId: string,
  status = 'success'
): Record<string, unknown> {
  return {
    type: 'toolRequest',
    id: requestId,
    toolCall: {
      status,
      value: { name, arguments: args },
    },
  };
}

function makeToolResponse(
  requestId: string,
  resultText: string,
  isError = false
): Record<string, unknown> {
  return {
    type: 'toolResponse',
    id: requestId,
    toolResult: {
      status: isError ? 'error' : 'success',
      value: {
        content: [{ text: resultText }],
      },
    },
  };
}

// ── buildToolResponseMap ─────────────────────────────────────────────

describe('buildToolResponseMap', () => {
  it('builds a map from tool response IDs to result text', () => {
    const messages: Message[] = [
      makeMsg('assistant', [
        makeToolRequest('developer__shell', { command: 'ls' }, 'req-1'),
      ]),
      makeMsg('user', [
        makeToolResponse('req-1', 'file1.txt\nfile2.txt'),
      ]),
    ];

    const map = buildToolResponseMap(messages);
    expect(map.size).toBe(1);
    expect(map.get('req-1')).toEqual({
      resultText: 'file1.txt\nfile2.txt',
      isError: false,
      errorMessage: undefined,
    });
  });

  it('detects error responses', () => {
    const messages: Message[] = [
      makeMsg('user', [
        makeToolResponse('req-2', 'Permission denied', true),
      ]),
    ];

    const map = buildToolResponseMap(messages);
    expect(map.get('req-2')).toEqual({
      resultText: 'Permission denied',
      isError: true,
      errorMessage: 'Permission denied',
    });
  });

  it('handles multiple tool responses in one message', () => {
    const messages: Message[] = [
      makeMsg('user', [
        makeToolResponse('req-a', 'result-a'),
        makeToolResponse('req-b', 'result-b'),
      ]),
    ];

    const map = buildToolResponseMap(messages);
    expect(map.size).toBe(2);
    expect(map.get('req-a')?.resultText).toBe('result-a');
    expect(map.get('req-b')?.resultText).toBe('result-b');
  });

  it('ignores assistant messages (only user messages have tool responses)', () => {
    const messages: Message[] = [
      makeMsg('assistant', [
        makeToolResponse('req-x', 'should be ignored'),
      ]),
    ];

    const map = buildToolResponseMap(messages);
    expect(map.size).toBe(0);
  });

  it('handles messages with no tool responses', () => {
    const messages: Message[] = [
      makeMsg('user', [{ type: 'text', text: 'hello' }]),
    ];

    const map = buildToolResponseMap(messages);
    expect(map.size).toBe(0);
  });

  it('handles empty messages array', () => {
    const map = buildToolResponseMap([]);
    expect(map.size).toBe(0);
  });
});

// ── extractActivityEntries ──────────────────────────────────────────

describe('extractActivityEntries', () => {
  it('extracts tool entries from assistant messages', () => {
    const messages: Message[] = [
      makeMsg('assistant', [
        makeToolRequest('developer__shell', { command: 'ls -la' }, 'req-1'),
      ]),
    ];

    const entries = extractActivityEntries(messages, false);
    expect(entries).toHaveLength(1);
    expect(entries[0].kind).toBe('tool');
    const tool = entries[0] as ToolActivityEntry;
    expect(tool.toolName).toBe('developer__shell');
    expect(tool.description).toContain('ls -la');
    expect(tool.isActive).toBe(false);
  });

  it('pairs tool requests with their responses', () => {
    const messages: Message[] = [
      makeMsg('assistant', [
        makeToolRequest('developer__shell', { command: 'pwd' }, 'req-1'),
      ]),
      makeMsg('user', [
        makeToolResponse('req-1', '/home/user/project'),
      ]),
    ];

    const entries = extractActivityEntries(messages, false);
    expect(entries).toHaveLength(1);
    const tool = entries[0] as ToolActivityEntry;
    expect(tool.toolResult).toBe('/home/user/project');
    expect(tool.isError).toBe(false);
  });

  it('pairs tool requests with error responses', () => {
    const messages: Message[] = [
      makeMsg('assistant', [
        makeToolRequest('developer__shell', { command: 'rm -rf /' }, 'req-1'),
      ]),
      makeMsg('user', [
        makeToolResponse('req-1', 'Operation not permitted', true),
      ]),
    ];

    const entries = extractActivityEntries(messages, false);
    const tool = entries[0] as ToolActivityEntry;
    expect(tool.isError).toBe(true);
    expect(tool.errorMessage).toBe('Operation not permitted');
  });

  it('extracts thinking entries from text + toolRequest messages', () => {
    const messages: Message[] = [
      makeMsg('assistant', [
        { type: 'text', text: 'Let me analyze the codebase structure first. I need to check the component hierarchy.' },
        makeToolRequest('developer__shell', { command: 'ls src/' }, 'req-1'),
      ]),
    ];

    const entries = extractActivityEntries(messages, false);
    expect(entries).toHaveLength(2);
    expect(entries[0].kind).toBe('thinking');
    expect(entries[0].description).toContain('analyze the codebase');
    expect(entries[1].kind).toBe('tool');
  });

  it('does NOT create thinking entries for pure text messages (no tool requests)', () => {
    const messages: Message[] = [
      makeMsg('assistant', [
        { type: 'text', text: 'Here is my final analysis of the codebase. The architecture looks solid.' },
      ]),
    ];

    const entries = extractActivityEntries(messages, false);
    expect(entries).toHaveLength(0);
  });

  it('handles multiple tool requests in one assistant message', () => {
    const messages: Message[] = [
      makeMsg('assistant', [
        makeToolRequest('developer__shell', { command: 'ls' }, 'req-1'),
        makeToolRequest('developer__text_editor', { path: '/tmp/test.txt', command: 'view' }, 'req-2'),
      ]),
      makeMsg('user', [
        makeToolResponse('req-1', 'file1.txt'),
        makeToolResponse('req-2', 'file contents here'),
      ]),
    ];

    const entries = extractActivityEntries(messages, false);
    expect(entries).toHaveLength(2);
    expect((entries[0] as ToolActivityEntry).toolName).toBe('developer__shell');
    expect((entries[0] as ToolActivityEntry).toolResult).toBe('file1.txt');
    expect((entries[1] as ToolActivityEntry).toolName).toBe('developer__text_editor');
    expect((entries[1] as ToolActivityEntry).toolResult).toBe('file contents here');
  });

  it('marks last message tool requests as active during streaming', () => {
    const messages: Message[] = [
      makeMsg('assistant', [
        makeToolRequest('developer__shell', { command: 'done' }, 'req-1', 'success'),
      ]),
      makeMsg('assistant', [
        makeToolRequest('developer__shell', { command: 'running...' }, 'req-2', 'pending'),
      ]),
    ];

    const entries = extractActivityEntries(messages, true);
    expect(entries).toHaveLength(2);
    expect(entries[0].isActive).toBe(false); // first msg, not last
    expect(entries[1].isActive).toBe(true); // last msg, streaming, pending
  });

  it('does NOT mark completed tools as active during streaming', () => {
    const messages: Message[] = [
      makeMsg('assistant', [
        makeToolRequest('developer__shell', { command: 'done' }, 'req-1', 'success'),
      ]),
    ];

    const entries = extractActivityEntries(messages, true);
    expect(entries).toHaveLength(1);
    expect(entries[0].isActive).toBe(false);
  });

  it('skips thinking text during streaming on last message', () => {
    const messages: Message[] = [
      makeMsg('assistant', [
        { type: 'text', text: 'This thinking text should be skipped during streaming.' },
        makeToolRequest('developer__shell', { command: 'ls' }, 'req-1', 'pending'),
      ]),
    ];

    const entries = extractActivityEntries(messages, true);
    expect(entries).toHaveLength(1);
    expect(entries[0].kind).toBe('tool');
  });

  it('includes tool args in the entry', () => {
    const args = { path: '/home/user/file.ts', command: 'view' };
    const messages: Message[] = [
      makeMsg('assistant', [
        makeToolRequest('developer__text_editor', args, 'req-1'),
      ]),
    ];

    const entries = extractActivityEntries(messages, false);
    const tool = entries[0] as ToolActivityEntry;
    expect(tool.toolArgs).toEqual(args);
  });

  it('handles a realistic multi-turn conversation', () => {
    const messages: Message[] = [
      // Turn 1: assistant calls shell
      makeMsg('assistant', [
        { type: 'text', text: 'I will start by examining the project structure. This will help me understand the codebase layout.' },
        makeToolRequest('developer__shell', { command: 'ls -la' }, 'req-1'),
      ]),
      // Turn 1 response
      makeMsg('user', [makeToolResponse('req-1', 'total 42\ndrwxr-xr-x src/')]),
      // Turn 2: assistant calls two tools
      makeMsg('assistant', [
        { type: 'text', text: 'Now let me look at the component files. I also need to check the package config.' },
        makeToolRequest('developer__shell', { command: 'ls src/components/' }, 'req-2'),
        makeToolRequest('developer__text_editor', { path: 'package.json', command: 'view' }, 'req-3'),
      ]),
      // Turn 2 responses
      makeMsg('user', [
        makeToolResponse('req-2', 'Button.tsx\nHeader.tsx'),
        makeToolResponse('req-3', '{"name": "my-app"}'),
      ]),
      // Turn 3: final text (pure text, no tools)
      makeMsg('assistant', [
        { type: 'text', text: 'Based on my analysis, the project has a clean component structure.' },
      ]),
    ];

    const entries = extractActivityEntries(messages, false);

    // Turn 1: 1 thinking + 1 tool
    expect(entries[0].kind).toBe('thinking');
    expect(entries[0].description).toContain('examining the project structure');
    expect(entries[1].kind).toBe('tool');
    expect((entries[1] as ToolActivityEntry).toolResult).toBe('total 42\ndrwxr-xr-x src/');

    // Turn 2: 1 thinking + 2 tools
    expect(entries[2].kind).toBe('thinking');
    expect(entries[3].kind).toBe('tool');
    expect((entries[3] as ToolActivityEntry).toolResult).toBe('Button.tsx\nHeader.tsx');
    expect(entries[4].kind).toBe('tool');
    expect((entries[4] as ToolActivityEntry).toolResult).toBe('{"name": "my-app"}');

    // Turn 3: pure text, no entries (it's the final answer, shown in main panel)
    expect(entries).toHaveLength(5);
  });

  it('handles empty messages array', () => {
    const entries = extractActivityEntries([], false);
    expect(entries).toHaveLength(0);
  });

  it('skips user messages entirely', () => {
    const messages: Message[] = [
      makeMsg('user', [{ type: 'text', text: 'Please analyze the code' }]),
    ];

    const entries = extractActivityEntries(messages, false);
    expect(entries).toHaveLength(0);
  });

  it('generates human-readable descriptions for known tools', () => {
    const messages: Message[] = [
      makeMsg('assistant', [
        makeToolRequest('developer__shell', { command: 'cargo build' }, 'r1'),
        makeToolRequest('developer__text_editor', { path: '/src/main.rs', command: 'view' }, 'r2'),
        makeToolRequest('developer__analyze', { path: '/src' }, 'r3'),
        makeToolRequest('apps__create_app', { prd: 'A todo app' }, 'r4'),
      ]),
    ];

    const entries = extractActivityEntries(messages, false);
    expect((entries[0] as ToolActivityEntry).description).toContain('cargo build');
    expect((entries[1] as ToolActivityEntry).description).toContain('main.rs');
    expect((entries[2] as ToolActivityEntry).description).toContain('src');
    expect((entries[3] as ToolActivityEntry).description).toContain('todo app');
  });
});
