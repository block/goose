import { Brain, Wrench, X } from 'lucide-react';
import { useEffect, useMemo, useRef } from 'react';
import type { Message } from '../../api';
import { useReasoningDetail } from '../../contexts/ReasoningDetailContext';
import { cn } from '../../utils';
import { Badge } from '../ui/atoms/Badge';
import { StatusDot } from '../ui/atoms/StatusDot';
import { ScrollArea } from '../ui/atoms/scroll-area';
import { ActivityStep, ThinkingEntry } from '../ui/molecules/ActivityStep';
import MarkdownContent from './MarkdownContent';

// ── Helpers ─────────────────────────────────────────────────────────

function snakeToTitle(s: string): string {
  return s
    .split('_')
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join(' ');
}

function getToolName(fullName: string): string {
  const parts = fullName.split('__');
  return parts[parts.length - 1] || fullName;
}

function describeToolArgs(name: string, args: Record<string, unknown>): string {
  const toolName = getToolName(name);
  const str = (v: unknown): string => (typeof v === 'string' ? v : JSON.stringify(v));

  switch (toolName) {
    case 'text_editor':
      if (args.command === 'write' && args.path) return `writing ${str(args.path)}`;
      if (args.command === 'view' && args.path) return `reading ${str(args.path)}`;
      if (args.command === 'str_replace' && args.path) return `editing ${str(args.path)}`;
      if (args.command === 'insert' && args.path) return `inserting in ${str(args.path)}`;
      if (args.path) return `${str(args.command)} ${str(args.path)}`;
      break;
    case 'shell':
      if (args.command) {
        const cmd = str(args.command);
        return `running ${cmd.length > 80 ? `${cmd.slice(0, 77)}…` : cmd}`;
      }
      break;
    case 'analyze':
      if (args.path) return `analyzing ${str(args.path)}`;
      break;
    case 'image_processor':
      if (args.path) return `processing image ${str(args.path)}`;
      break;
    case 'screen_capture':
      return args.window_title ? `capturing "${str(args.window_title)}"` : 'capturing screen';
    case 'create_app':
    case 'iterate_app':
      if (args.name)
        return `${toolName === 'create_app' ? 'creating' : 'updating'} app ${str(args.name)}`;
      break;
    default:
      break;
  }
  const display = snakeToTitle(toolName);
  const firstStr = Object.values(args).find((v) => typeof v === 'string');
  if (firstStr) {
    const val = str(firstStr);
    return `${display}: ${val.length > 60 ? `${val.slice(0, 57)}…` : val}`;
  }
  return display;
}

// ── Activity Entry types ────────────────────────────────────────────

export interface ToolActivityEntry {
  kind: 'tool';
  id: string;
  toolName: string;
  description: string;
  isActive: boolean;
  toolArgs?: Record<string, unknown>;
  toolResult?: string;
  isError: boolean;
  errorMessage?: string;
}

interface ThinkingActivityEntry {
  kind: 'thinking';
  id: string;
  description: string;
  isActive: boolean;
}

type ActivityEntry = ToolActivityEntry | ThinkingActivityEntry;

// ── Tool response pairing ───────────────────────────────────────────

type ToolResponseInfo = { resultText: string; isError: boolean; errorMessage?: string };
type ToolResponseMap = Map<string, ToolResponseInfo>;

export function buildToolResponseMap(messages: Message[]): ToolResponseMap {
  const map: ToolResponseMap = new Map();

  for (const msg of messages) {
    if (msg.role !== 'user') continue;
    if (!Array.isArray(msg.content)) continue;

    for (const c of msg.content) {
      if (typeof c !== 'object' || c === null || !('type' in c)) continue;
      if ((c as { type: string }).type !== 'toolResponse') continue;

      const resp = c as {
        id?: string;
        toolResult?: {
          status?: string;
          value?: { content?: Array<{ text?: string }> };
        };
      };

      const id = resp.id;
      if (!id) continue;

      const status = resp.toolResult?.status;
      const isError = status === 'error';
      const contentArr = resp.toolResult?.value?.content || [];

      const textParts: string[] = [];
      for (const item of contentArr) {
        if (item && typeof item.text === 'string') {
          textParts.push(item.text);
        }
      }

      const resultText = textParts.join('\n');
      map.set(id, {
        resultText,
        isError,
        errorMessage: isError ? resultText : undefined,
      });
    }
  }

  return map;
}

// ── Extract activity entries ────────────────────────────────────────

function firstSentence(text: string): string | null {
  const cleaned = text
    .replace(/```[\s\S]*?```/g, '')
    .replace(/[#*_`~>]/g, '')
    .replace(/\n+/g, ' ')
    .trim();
  const match = cleaned.match(/^(.+?[.!?:—])\s/);
  const sentence = match ? match[1].trim() : null;
  if (sentence && sentence.length > 5) {
    return sentence.length > 120 ? `${sentence.slice(0, 117)}…` : sentence;
  }
  return null;
}

export function extractActivityEntries(messages: Message[], isStreaming: boolean): ActivityEntry[] {
  const entries: ActivityEntry[] = [];
  const responseMap = buildToolResponseMap(messages);

  for (let i = 0; i < messages.length; i++) {
    const msg = messages[i];
    if (msg.role !== 'assistant') continue;

    const content = msg.content;
    if (!Array.isArray(content)) continue;

    const isLastMsg = i === messages.length - 1;
    const isStreamingMsg = isStreaming && isLastMsg;

    const hasToolRequest = content.some(
      (c) =>
        typeof c === 'object' && c !== null && 'type' in c && (c as { type: string }).type === 'toolRequest'
    );

    for (const c of content) {
      if (typeof c !== 'object' || c === null || !('type' in c)) continue;
      const cTyped = c as { type: string; text?: string; id?: string; toolCall?: unknown };

      if (cTyped.type === 'text' && hasToolRequest) {
        const text = typeof cTyped.text === 'string' ? cTyped.text.trim() : '';

        if (isStreamingMsg) continue;

        if (text.length > 10) {
          const sentence = firstSentence(text);
          if (sentence) {
            entries.push({
              kind: 'thinking',
              id: `think-${msg.id || i}`,
              description: sentence,
              isActive: false,
            });
          }
        }
      }

      if (cTyped.type === 'toolRequest') {
        const toolCall = cTyped.toolCall as {
          status?: string;
          value?: { name?: string; arguments?: Record<string, unknown> };
        } | undefined;
        const requestId = cTyped.id;
        const name = toolCall?.value?.name || 'unknown';
        const args = (toolCall?.value?.arguments || {}) as Record<string, unknown>;
        const isPending = !toolCall?.status || toolCall.status === 'pending';
        const isActive = isStreamingMsg && isPending;

        const pairedResponse = requestId ? responseMap.get(requestId) : undefined;

        entries.push({
          kind: 'tool',
          id: `tool-${requestId || `${i}-${entries.length}`}`,
          toolName: name,
          description: describeToolArgs(name, args),
          isActive,
          toolArgs: args,
          toolResult: pairedResponse?.resultText,
          isError: pairedResponse?.isError ?? false,
          errorMessage: pairedResponse?.errorMessage,
        });
      }
    }
  }

  return entries;
}

// ── Component ───────────────────────────────────────────────────────

export default function ReasoningDetailPanel() {
  const { detail, panelDetail, isOpen, closeDetail } = useReasoningDetail();
  const bottomRef = useRef<HTMLDivElement>(null);
  const rafId = useRef<number | null>(null);

  const isWorkBlock = panelDetail !== null && panelDetail.type === 'workblock';
  const isReasoning = panelDetail !== null && panelDetail.type === 'reasoning';

  const workBlockData = isWorkBlock ? panelDetail.data : null;
  const reasoningData = isReasoning ? panelDetail.data : null;

  const isLiveStreaming = workBlockData?.isStreaming ?? false;

  const title = workBlockData
    ? workBlockData.title || 'Work Block'
    : detail?.title || 'Reasoning';

  const showAgentBadge =
    workBlockData && workBlockData.agentName && workBlockData.agentName !== 'default';

  const activityEntries = useMemo(() => {
    if (!workBlockData) return [];
    return extractActivityEntries(workBlockData.messages, !!isLiveStreaming);
  }, [workBlockData, isLiveStreaming]);

  useEffect(() => {
    if (!isLiveStreaming || !bottomRef.current) return;

    const scroll = () => {
      bottomRef.current?.scrollIntoView({ behavior: 'smooth', block: 'end' });
      rafId.current = requestAnimationFrame(scroll);
    };
    rafId.current = requestAnimationFrame(scroll);

    return () => {
      if (rafId.current) cancelAnimationFrame(rafId.current);
      rafId.current = null;
    };
  }, [isLiveStreaming]);

  return (
    <div
      className={cn(
        'h-full border-l border-border-default bg-background-default flex flex-col transition-[width,opacity] duration-300 ease-in-out overflow-hidden',
        isOpen ? 'w-[400px] min-w-[400px] opacity-100' : 'w-0 min-w-0 opacity-0'
      )}
    >
      {isOpen && (
        <>
          {/* Header */}
          <div className="flex items-center justify-between px-4 py-3 border-b border-border-default shrink-0">
            <div className="flex items-center gap-2 min-w-0">
              {isWorkBlock ? (
                <Wrench
                  size={16}
                  className={cn(
                    'shrink-0',
                    isLiveStreaming ? 'text-blue-400 animate-pulse' : 'text-text-muted'
                  )}
                />
              ) : (
                <Brain
                  size={16}
                  className={cn(
                    'shrink-0',
                    isLiveStreaming ? 'text-amber-400 animate-pulse' : 'text-text-muted'
                  )}
                />
              )}
              <h3 className="text-sm font-medium text-text-default truncate">{title}</h3>
              {showAgentBadge && workBlockData && (
                <Badge variant="default" size="sm" className="ml-1">
                  {workBlockData.agentName}
                  {workBlockData.modeName && workBlockData.modeName !== 'default'
                    ? ` / ${workBlockData.modeName}`
                    : ''}
                </Badge>
              )}
            </div>
            <button
              type="button"
              onClick={closeDetail}
              className="p-1 rounded-md hover:bg-background-muted transition-colors shrink-0 cursor-pointer"
            >
              <X size={16} className="text-text-muted" />
            </button>
          </div>

          {/* Content */}
          <ScrollArea className="flex-1 min-h-0">
            {workBlockData ? (
              <div className="px-3 py-2 space-y-0.5">
                {workBlockData.toolCount > 0 && (
                  <div className="flex items-center gap-2 text-xs text-text-muted px-2 py-1.5 mb-1 border-b border-border-default">
                    <StatusDot status={isLiveStreaming ? 'active' : 'completed'} />
                    <span>
                      {workBlockData.toolCount} tool
                      {workBlockData.toolCount !== 1 ? 's' : ''} used
                    </span>
                  </div>
                )}
                {activityEntries.map((entry) =>
                  entry.kind === 'thinking' ? (
                    <ThinkingEntry key={entry.id} text={entry.description} />
                  ) : (
                    <ActivityStep
                      key={entry.id}
                      description={entry.description}
                      toolName={entry.toolName}
                      isActive={entry.isActive}
                      toolArgs={entry.toolArgs}
                      toolResult={entry.toolResult}
                      isError={entry.isError}
                      errorMessage={entry.errorMessage}
                    />
                  )
                )}
                <div ref={bottomRef} />
              </div>
            ) : reasoningData ? (
              <div className="text-sm text-text-muted/90 px-4 py-4">
                <MarkdownContent content={detail?.content ?? ''} />
                <div ref={bottomRef} />
              </div>
            ) : null}
          </ScrollArea>
        </>
      )}
    </div>
  );
}
