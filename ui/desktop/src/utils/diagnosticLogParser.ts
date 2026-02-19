/**
 * Diagnostic Log Parser
 *
 * Parses JSONL diagnostic logs from goose sessions and categorizes every
 * message/event into the same UI rendering zones used by the desktop app.
 *
 * The classification mirrors the rendering pipeline:
 *   assistantWorkBlocks.ts → ProgressiveMessageList.tsx → GooseMessage/WorkBlockIndicator
 *
 * Categories map to 4 UI zones:
 *   MAIN PANEL  → USER_INPUT, ASSISTANT_TEXT, STREAMING_CHUNK
 *   WORK BLOCK  → TOOL_REQUEST, TOOL_RESULT, INTERMEDIATE_TEXT
 *   REASONING   → THINKING
 *   HIDDEN      → SYSTEM_INFO, TITLE_GENERATION, USAGE_STATS
 */

import type { Message, MessageContent } from '../api';

// ── Enums ────────────────────────────────────────────────────────────

export enum Zone {
  MAIN_PANEL = 'main_panel',
  WORK_BLOCK = 'work_block',
  REASONING = 'reasoning',
  HIDDEN = 'hidden',
}

export enum Category {
  USER_INPUT = 'user_input',
  ASSISTANT_TEXT = 'assistant_text',
  STREAMING_CHUNK = 'streaming_chunk',
  TOOL_REQUEST = 'tool_request',
  TOOL_RESULT = 'tool_result',
  INTERMEDIATE_TEXT = 'intermediate_text',
  THINKING = 'thinking',
  SYSTEM_INFO = 'system_info',
  TITLE_GENERATION = 'title_generation',
  USAGE_STATS = 'usage_stats',
}

const CATEGORY_ZONE: Record<Category, Zone> = {
  [Category.USER_INPUT]: Zone.MAIN_PANEL,
  [Category.ASSISTANT_TEXT]: Zone.MAIN_PANEL,
  [Category.STREAMING_CHUNK]: Zone.MAIN_PANEL,
  [Category.TOOL_REQUEST]: Zone.WORK_BLOCK,
  [Category.TOOL_RESULT]: Zone.WORK_BLOCK,
  [Category.INTERMEDIATE_TEXT]: Zone.WORK_BLOCK,
  [Category.THINKING]: Zone.REASONING,
  [Category.SYSTEM_INFO]: Zone.HIDDEN,
  [Category.TITLE_GENERATION]: Zone.HIDDEN,
  [Category.USAGE_STATS]: Zone.HIDDEN,
};

export function zoneOf(category: Category): Zone {
  return CATEGORY_ZONE[category];
}

// ── Data types ───────────────────────────────────────────────────────

export interface CategorizedItem {
  category: Category;
  zone: Zone;
  source: string;
  role: string;
  summary: string;
  toolName?: string;
  text?: string;
  isStreaming?: boolean;
  tokenUsage?: { input_tokens: number; output_tokens: number; total_tokens: number };
}

export interface TimelineWorkBlock {
  type: 'work_block';
  toolCalls: number;
  toolResults: number;
  intermediateText: number;
  toolNames: string[];
  items: CategorizedItem[];
}

export interface TimelineMessage {
  type: 'message';
  item: CategorizedItem;
  chunkCount?: number;
}

export type TimelineEntry = TimelineWorkBlock | TimelineMessage;

export interface ParsedSession {
  conversationItems: CategorizedItem[];
  responseItems: CategorizedItem[];
  titleItems: CategorizedItem[];
  timeline: TimelineEntry[];
  zoneCounts: Record<Zone, number>;
  categoryCounts: Record<Category, number>;
}

// ── Raw log types (JSONL structure) ──────────────────────────────────

interface RawLogContent {
  type: string;
  text?: string;
  name?: string;
  id?: string;
  toolCall?: {
    status?: string;
    value?: { name?: string; arguments?: Record<string, unknown> };
  };
  data?: Record<string, unknown>;
}

interface RawLogMessage {
  role: string;
  content: string | RawLogContent[];
}

interface RawLogInput {
  model?: string;
  system?: string;
  messages?: RawLogMessage[];
  tools?: unknown[];
  stream?: boolean;
  input?: {
    system?: string;
    messages?: RawLogMessage[];
  };
}

interface RawLogResponse {
  data?: {
    id?: string;
    role?: string;
    content?: RawLogContent[];
  } | null;
  usage?: {
    input_tokens?: number;
    output_tokens?: number;
    total_tokens?: number;
  };
}

// ── Content helpers (mirror assistantWorkBlocks.ts) ──────────────────

function normalizeContent(content: string | RawLogContent[]): RawLogContent[] {
  if (typeof content === 'string') {
    return [{ type: 'text', text: content }];
  }
  return Array.isArray(content) ? content : [];
}

function isInfoMsg(content: RawLogContent[]): boolean {
  return content.some(
    (c) =>
      c.type === 'text' &&
      typeof c.text === 'string' &&
      (c.text.trim().startsWith('<info-msg>') || c.text.slice(0, 60).includes('It is currently'))
  );
}

function hasOnlyToolResponses(content: RawLogContent[]): boolean {
  if (content.length === 0) return false;
  return content.every((c) => c.type === 'tool_result' || c.type === 'toolResponse');
}

function hasToolRequests(content: RawLogContent[]): boolean {
  return content.some((c) => c.type === 'tool_use' || c.type === 'toolRequest');
}

function hasDisplayText(content: RawLogContent[]): boolean {
  return content.some(
    (c) =>
      c.type === 'text' &&
      typeof c.text === 'string' &&
      c.text.trim().length > 0 &&
      !c.text.trim().startsWith('<info-msg>')
  );
}

function hasThinking(content: RawLogContent[]): boolean {
  return content.some((c) => c.type === 'thinking' || c.type === 'redactedThinking');
}

function countToolRequests(content: RawLogContent[]): number {
  return content.filter((c) => c.type === 'tool_use' || c.type === 'toolRequest').length;
}

function getToolNames(content: RawLogContent[]): string[] {
  const names: string[] = [];
  for (const c of content) {
    if (c.type === 'tool_use' && c.name) {
      names.push(c.name);
    } else if (c.type === 'toolRequest' && c.toolCall?.value?.name) {
      names.push(c.toolCall.value.name);
    }
  }
  return names;
}

function textPreview(content: RawLogContent[], maxLen = 120): string {
  const parts: string[] = [];
  for (const c of content) {
    if (c.type === 'text' && typeof c.text === 'string') {
      const t = c.text.trim();
      if (t && !t.startsWith('<info-msg>')) parts.push(t);
    }
  }
  const combined = parts.join(' ');
  return combined.length > maxLen ? combined.slice(0, maxLen - 3) + '...' : combined;
}

// ── User message classification ──────────────────────────────────────

function isRealUserMessage(
  msgIdx: number,
  messages: RawLogMessage[]
): boolean {
  const msg = messages[msgIdx];
  if (msg.role !== 'user') return false;

  const content = normalizeContent(msg.content);
  if (hasOnlyToolResponses(content)) return false;

  for (let i = msgIdx - 1; i >= 0; i--) {
    const prev = messages[i];
    if (prev.role === 'assistant') {
      return !hasToolRequests(normalizeContent(prev.content));
    }
    if (prev.role === 'user') {
      const prevContent = normalizeContent(prev.content);
      if (hasOnlyToolResponses(prevContent)) continue;
      return true;
    }
  }
  return true;
}

// ── Work block identification (port of identifyWorkBlocks) ───────────

interface LogWorkBlock {
  indices: Set<number>;
  finalIndex: number;
  toolCount: number;
}

function identifyLogWorkBlocks(messages: RawLogMessage[]): Map<number, LogWorkBlock> {
  const result = new Map<number, LogWorkBlock>();
  const runs: Array<{ start: number; end: number }> = [];
  let blockStart = -1;

  for (let i = 0; i < messages.length; i++) {
    const isAssistant = messages[i].role === 'assistant';
    if (isAssistant && blockStart === -1) {
      blockStart = i;
    } else if (!isAssistant && blockStart !== -1) {
      if (isRealUserMessage(i, messages)) {
        runs.push({ start: blockStart, end: i - 1 });
        blockStart = -1;
      }
    }
  }
  if (blockStart !== -1) {
    runs.push({ start: blockStart, end: messages.length - 1 });
  }

  for (const run of runs) {
    const assistantIndices: number[] = [];
    for (let i = run.start; i <= run.end; i++) {
      if (messages[i].role === 'assistant') assistantIndices.push(i);
    }

    if (assistantIndices.length <= 1) continue;

    // Find final answer (prefer pure text, fallback to text+tools)
    let finalIdx = -1;
    let textWithToolsIdx = -1;

    for (let i = assistantIndices.length - 1; i >= 0; i--) {
      const idx = assistantIndices[i];
      const content = normalizeContent(messages[idx].content);
      if (!hasDisplayText(content)) continue;
      if (!hasToolRequests(content)) {
        finalIdx = idx;
        break;
      } else if (textWithToolsIdx === -1) {
        textWithToolsIdx = idx;
      }
    }
    if (finalIdx === -1 && textWithToolsIdx !== -1) {
      finalIdx = textWithToolsIdx;
    }

    let totalTools = 0;
    const allBlockIndices = new Set<number>();
    for (let i = run.start; i <= run.end; i++) {
      if (i !== finalIdx) {
        allBlockIndices.add(i);
        if (messages[i].role === 'assistant') {
          totalTools += countToolRequests(normalizeContent(messages[i].content));
        }
      }
    }

    if (allBlockIndices.size === 0) continue;

    const block: LogWorkBlock = {
      indices: allBlockIndices,
      finalIndex: finalIdx,
      toolCount: totalTools,
    };

    for (const idx of allBlockIndices) {
      result.set(idx, block);
    }
  }

  return result;
}

// ── Title generation detection ───────────────────────────────────────

function isTitleGeneration(entry: RawLogInput): boolean {
  const input = entry.input ?? entry;
  const sys = input.system ?? '';
  const msgs = input.messages ?? [];

  // Collect all text from system prompt and messages
  const allTexts: string[] = [];
  if (typeof sys === 'string') allTexts.push(sys);
  for (const msg of msgs) {
    if (typeof msg.content === 'string') {
      allTexts.push(msg.content);
    } else if (Array.isArray(msg.content)) {
      for (const c of msg.content) {
        if (typeof c === 'object' && c.text) allTexts.push(c.text);
      }
    }
  }

  const combined = allTexts.join(' ').toLowerCase();
  const titleSignals = ['first few user messages', 'generate a title', 'short title', 'summarize'];

  // Title generation: short system prompt + few messages + signal phrases
  if (typeof sys === 'string' && sys.length < 200 && msgs.length <= 2) {
    if (titleSignals.some((sig) => combined.includes(sig))) return true;
  }

  // Explicit title generation phrases in any text
  if (combined.includes('first few user messages')) return true;

  return false;
}

// ── Message categorization ───────────────────────────────────────────

function categorizeInputMessage(
  msg: RawLogMessage,
  msgIdx: number,
  allMsgs: RawLogMessage[],
  source: string,
  workBlocks: Map<number, LogWorkBlock>
): CategorizedItem {
  const content = normalizeContent(msg.content);
  const inBlock = workBlocks.has(msgIdx);

  if (msg.role === 'user') {
    if (isInfoMsg(content) && !hasDisplayText(content)) {
      return { category: Category.SYSTEM_INFO, zone: Zone.HIDDEN, source, role: msg.role, summary: 'System timestamp injection' };
    }
    if (hasOnlyToolResponses(content)) {
      return { category: Category.TOOL_RESULT, zone: Zone.WORK_BLOCK, source, role: msg.role, summary: `Tool result (${content.length} items)` };
    }
    if (isRealUserMessage(msgIdx, allMsgs)) {
      const preview = textPreview(content);
      return { category: Category.USER_INPUT, zone: Zone.MAIN_PANEL, source, role: msg.role, summary: preview || 'User message', text: preview };
    }
    return { category: Category.TOOL_RESULT, zone: Zone.WORK_BLOCK, source, role: msg.role, summary: 'Summarized tool result' };
  }

  if (msg.role === 'assistant') {
    const hasText = hasDisplayText(content);
    const hasTools = hasToolRequests(content);
    const hasThink = hasThinking(content);

    if (hasThink) {
      return { category: Category.THINKING, zone: Zone.REASONING, source, role: msg.role, summary: 'Chain-of-thought reasoning' };
    }

    if (hasTools && hasText) {
      const toolNames = getToolNames(content);
      const preview = textPreview(content, 80);
      if (inBlock) {
        return { category: Category.INTERMEDIATE_TEXT, zone: Zone.WORK_BLOCK, source, role: msg.role, summary: `Thinking: ${preview}`, toolName: toolNames[0], text: preview };
      }
      return { category: Category.ASSISTANT_TEXT, zone: Zone.MAIN_PANEL, source, role: msg.role, summary: preview || 'Assistant response', text: preview };
    }

    if (hasTools) {
      const toolNames = getToolNames(content);
      const count = countToolRequests(content);
      return { category: Category.TOOL_REQUEST, zone: Zone.WORK_BLOCK, source, role: msg.role, summary: `${toolNames.join(', ')} (${count} call${count > 1 ? 's' : ''})`, toolName: toolNames[0] };
    }

    if (hasText) {
      const preview = textPreview(content);
      if (inBlock) {
        return { category: Category.INTERMEDIATE_TEXT, zone: Zone.WORK_BLOCK, source, role: msg.role, summary: `Intermediate: ${preview}`, text: preview };
      }
      return { category: Category.ASSISTANT_TEXT, zone: Zone.MAIN_PANEL, source, role: msg.role, summary: preview || 'Assistant response', text: preview };
    }

    return { category: Category.SYSTEM_INFO, zone: Zone.HIDDEN, source, role: msg.role, summary: 'Empty assistant message' };
  }

  return { category: Category.SYSTEM_INFO, zone: Zone.HIDDEN, source, role: msg.role, summary: `Unknown role: ${msg.role}` };
}

function categorizeResponse(
  lineData: RawLogResponse,
  source: string,
  isTitleGen: boolean
): CategorizedItem[] {
  const items: CategorizedItem[] = [];

  if (lineData.data === null || lineData.data === undefined) {
    if (lineData.usage) {
      items.push({
        category: Category.USAGE_STATS,
        zone: Zone.HIDDEN,
        source,
        role: 'system',
        summary: `in=${lineData.usage.input_tokens ?? '?'} out=${lineData.usage.output_tokens ?? '?'} total=${lineData.usage.total_tokens ?? '?'}`,
        tokenUsage: lineData.usage as CategorizedItem['tokenUsage'],
      });
    }
    return items;
  }

  const content = lineData.data.content ?? [];
  for (const c of content) {
    if (c.type === 'text' && typeof c.text === 'string') {
      if (isTitleGen) {
        items.push({ category: Category.TITLE_GENERATION, zone: Zone.HIDDEN, source, role: 'assistant', summary: `Title: ${c.text.slice(0, 80)}`, text: c.text });
      } else {
        items.push({ category: Category.STREAMING_CHUNK, zone: Zone.MAIN_PANEL, source, role: 'assistant', summary: `Chunk: ${c.text.slice(0, 60)}`, text: c.text, isStreaming: true });
      }
    } else if (c.type === 'toolRequest') {
      const name = c.toolCall?.value?.name ?? 'unknown';
      items.push({ category: Category.TOOL_REQUEST, zone: Zone.WORK_BLOCK, source, role: 'assistant', summary: `Tool: ${name}`, toolName: name });
    } else if (c.type === 'thinking' || c.type === 'redactedThinking') {
      items.push({ category: Category.THINKING, zone: Zone.REASONING, source, role: 'assistant', summary: 'Reasoning' });
    }
  }

  return items.length > 0 ? items : [{ category: Category.USAGE_STATS, zone: Zone.HIDDEN, source, role: 'system', summary: 'Response metadata' }];
}

// ── Timeline builder ─────────────────────────────────────────────────

function buildTimeline(
  conversationItems: CategorizedItem[],
  responseItems: CategorizedItem[]
): TimelineEntry[] {
  const timeline: TimelineEntry[] = [];
  let currentBlock: CategorizedItem[] = [];

  function flushBlock() {
    if (currentBlock.length === 0) return;
    const toolCalls = currentBlock.filter((i) => i.category === Category.TOOL_REQUEST).length;
    const toolResults = currentBlock.filter((i) => i.category === Category.TOOL_RESULT).length;
    const intermediate = currentBlock.filter((i) => i.category === Category.INTERMEDIATE_TEXT).length;
    const names = [...new Set(currentBlock.filter((i) => i.toolName).map((i) => i.toolName!))];

    timeline.push({
      type: 'work_block',
      toolCalls,
      toolResults,
      intermediateText: intermediate,
      toolNames: names,
      items: [...currentBlock],
    });
    currentBlock = [];
  }

  for (const item of conversationItems) {
    if (zoneOf(item.category) === Zone.WORK_BLOCK) {
      currentBlock.push(item);
    } else if (zoneOf(item.category) !== Zone.HIDDEN) {
      flushBlock();
      timeline.push({ type: 'message', item });
    }
  }
  flushBlock();

  // Accumulate streaming chunks into assembled messages
  let streamingChunks: CategorizedItem[] = [];
  let responseBlock: CategorizedItem[] = [];

  function flushStreaming() {
    if (streamingChunks.length === 0) return;
    const fullText = streamingChunks.map((c) => c.text ?? '').join('');
    timeline.push({
      type: 'message',
      item: {
        category: Category.ASSISTANT_TEXT,
        zone: Zone.MAIN_PANEL,
        source: streamingChunks[0].source,
        role: 'assistant',
        summary: fullText.length > 120 ? fullText.slice(0, 117) + '...' : fullText,
        text: fullText,
      },
      chunkCount: streamingChunks.length,
    });
    streamingChunks = [];
  }

  function flushResponseBlock() {
    if (responseBlock.length === 0) return;
    const toolCalls = responseBlock.filter((i) => i.category === Category.TOOL_REQUEST).length;
    const names = [...new Set(responseBlock.filter((i) => i.toolName).map((i) => i.toolName!))];
    timeline.push({
      type: 'work_block',
      toolCalls,
      toolResults: 0,
      intermediateText: 0,
      toolNames: names,
      items: [...responseBlock],
    });
    responseBlock = [];
  }

  for (const item of responseItems) {
    if (item.category === Category.STREAMING_CHUNK) {
      if (responseBlock.length > 0) flushResponseBlock();
      streamingChunks.push(item);
    } else if (zoneOf(item.category) === Zone.MAIN_PANEL) {
      flushStreaming();
      if (responseBlock.length > 0) flushResponseBlock();
      timeline.push({ type: 'message', item });
    } else if (zoneOf(item.category) === Zone.WORK_BLOCK) {
      flushStreaming();
      responseBlock.push(item);
    }
  }
  flushStreaming();
  flushResponseBlock();

  return timeline;
}

// ── JSONL parser ─────────────────────────────────────────────────────

interface FileData {
  inputMsgs: RawLogMessage[];
  items: CategorizedItem[];
  isTitleGen: boolean;
}

export function parseLogLines(lines: string[], filename: string): FileData {
  const result: FileData = { inputMsgs: [], items: [], isTitleGen: false };

  for (let lineNum = 0; lineNum < lines.length; lineNum++) {
    const raw = lines[lineNum].trim();
    if (!raw) continue;

    let entry: Record<string, unknown>;
    try {
      entry = JSON.parse(raw);
    } catch {
      continue;
    }

    if ('model' in entry || 'input' in entry) {
      const inputData = (entry.input ?? entry) as RawLogInput;
      result.inputMsgs = (inputData.messages ?? []) as RawLogMessage[];
      result.isTitleGen = isTitleGeneration(entry as RawLogInput);
    } else if ('data' in entry || 'usage' in entry) {
      const source = `${filename}:L${lineNum}`;
      const items = categorizeResponse(entry as RawLogResponse, source, result.isTitleGen);
      result.items.push(...items);
    }
  }

  return result;
}

export function parseSession(fileContents: Map<string, string[]>): ParsedSession {
  const parsed = new Map<string, FileData>();

  for (const [filename, lines] of fileContents) {
    parsed.set(filename, parseLogLines(lines, filename));
  }

  // Find the most complete non-title-gen file
  let bestFile: string | null = null;
  let bestCount = 0;
  for (const [filename, data] of parsed) {
    if (data.isTitleGen) continue;
    if (data.inputMsgs.length > bestCount) {
      bestCount = data.inputMsgs.length;
      bestFile = filename;
    }
  }

  // Categorize the canonical conversation
  const conversationItems: CategorizedItem[] = [];
  if (bestFile) {
    const msgs = parsed.get(bestFile)!.inputMsgs;
    const workBlocks = identifyLogWorkBlocks(msgs);
    for (let idx = 0; idx < msgs.length; idx++) {
      const source = `${bestFile}:msg[${idx}]`;
      conversationItems.push(categorizeInputMessage(msgs[idx], idx, msgs, source, workBlocks));
    }
  }

  // Collect response items
  const responseItems: CategorizedItem[] = [];
  const titleItems: CategorizedItem[] = [];
  for (const data of parsed.values()) {
    for (const item of data.items) {
      if (item.category === Category.TITLE_GENERATION) {
        titleItems.push(item);
      } else {
        responseItems.push(item);
      }
    }
  }

  const timeline = buildTimeline(conversationItems, responseItems);

  // Compute counts
  const allItems = [...conversationItems, ...responseItems, ...titleItems];
  const zoneCounts = { [Zone.MAIN_PANEL]: 0, [Zone.WORK_BLOCK]: 0, [Zone.REASONING]: 0, [Zone.HIDDEN]: 0 };
  const categoryCounts = Object.fromEntries(Object.values(Category).map((c) => [c, 0])) as Record<Category, number>;
  for (const item of allItems) {
    zoneCounts[item.zone]++;
    categoryCounts[item.category]++;
  }

  return { conversationItems, responseItems, titleItems, timeline, zoneCounts, categoryCounts };
}

// ── Conversion to UI Message[] for rendering ─────────────────────────

/**
 * Convert a parsed session's conversation into Message[] compatible with
 * the existing rendering pipeline (ProgressiveMessageList, GooseMessage, etc).
 *
 * This is used when loading diagnostic logs into the chat view.
 */
export function toMessages(session: ParsedSession): Message[] {
  const messages: Message[] = [];

  for (const item of session.conversationItems) {
    if (item.zone === Zone.HIDDEN) continue;

    const content: MessageContent[] = [];

    switch (item.category) {
      case Category.USER_INPUT:
        content.push({ type: 'text', text: item.text ?? item.summary });
        break;
      case Category.ASSISTANT_TEXT:
      case Category.INTERMEDIATE_TEXT:
        if (item.text) content.push({ type: 'text', text: item.text });
        if (item.toolName) {
          content.push({
            type: 'toolRequest',
            id: `diag-${messages.length}`,
            toolCall: { status: 'success', value: { name: item.toolName, arguments: {} } },
          } as MessageContent);
        }
        break;
      case Category.TOOL_REQUEST:
        content.push({
          type: 'toolRequest',
          id: `diag-${messages.length}`,
          toolCall: { status: 'success', value: { name: item.toolName ?? 'unknown', arguments: {} } },
        } as MessageContent);
        break;
      case Category.TOOL_RESULT:
        content.push({ type: 'toolResponse', id: `diag-${messages.length}` } as MessageContent);
        break;
      case Category.THINKING:
        content.push({ type: 'thinking', thinking: 'Chain-of-thought reasoning' } as MessageContent);
        break;
    }

    if (content.length > 0) {
      messages.push({
        role: item.role as 'user' | 'assistant',
        content,
        id: `diag-msg-${messages.length}`,
        created: Date.now() / 1000,
        metadata: { agentVisible: true, userVisible: true },
      });
    }
  }

  return messages;
}
