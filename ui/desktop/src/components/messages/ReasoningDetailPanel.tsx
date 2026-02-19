import { Brain, Wrench, X } from 'lucide-react';
import { useEffect, useMemo, useRef } from 'react';
import type { Message } from '../../api';
import { useReasoningDetail } from '../../contexts/ReasoningDetailContext';
import { cn } from '../../utils';
import { Badge } from '../ui/atoms/Badge';
import { StatusDot } from '../ui/atoms/StatusDot';
import { ScrollArea } from '../ui/atoms/scroll-area';
import { ActivityStep } from '../ui/molecules/ActivityStep';
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

interface ActivityEntry {
  kind: 'tool' | 'thinking';
  id: string;
  toolName?: string;
  description: string;
  isActive: boolean;
}

/** Extract a compact list of activity entries from work block messages. */
function extractActivityEntries(messages: Message[], isStreaming: boolean): ActivityEntry[] {
  const entries: ActivityEntry[] = [];

  for (let i = 0; i < messages.length; i++) {
    const msg = messages[i];
    if (msg.role !== 'assistant') continue;

    const content = msg.content;
    if (!Array.isArray(content)) continue;

    // Extract thinking text (only if there's also a tool call — pure text is the final answer)
    const hasToolRequest = content.some(
      (c) => typeof c === 'object' && c !== null && 'type' in c && c.type === 'toolRequest'
    );

    for (const c of content) {
      if (typeof c !== 'object' || c === null || !('type' in c)) continue;

      if (c.type === 'text' && hasToolRequest) {
        const text = 'text' in c && typeof c.text === 'string' ? c.text.trim() : '';
        if (text.length > 10) {
          // Extract first sentence as thinking summary
          const cleaned = text
            .replace(/```[\s\S]*?```/g, '')
            .replace(/[#*_`~>]/g, '')
            .trim();
          const firstLine = cleaned.split(/[.\n]/)[0]?.trim();
          if (firstLine && firstLine.length > 5) {
            entries.push({
              kind: 'thinking',
              id: `think-${msg.id || i}`,
              description: firstLine.length > 120 ? `${firstLine.slice(0, 117)}…` : firstLine,
              isActive: false,
            });
          }
        }
      }

      if (c.type === 'toolRequest') {
        const toolCall = (
          c as {
            toolCall?: {
              status?: string;
              value?: { name?: string; arguments?: Record<string, unknown> };
            };
          }
        ).toolCall;
        const name = toolCall?.value?.name || 'unknown';
        const args = (toolCall?.value?.arguments || {}) as Record<string, unknown>;
        const isLastMsg = i === messages.length - 1;
        const isPending = !toolCall?.status || toolCall.status === 'pending';
        const isActive = isStreaming && isLastMsg && isPending;

        entries.push({
          kind: 'tool',
          id: `tool-${(c as { id?: string }).id || `${i}-${entries.length}`}`,
          toolName: name,
          description: describeToolArgs(name, args),
          isActive,
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

  const isWorkBlock = panelDetail?.type === 'workblock';
  const isReasoning = panelDetail?.type === 'reasoning' || (!panelDetail && detail);

  const isWorkBlockStreaming = isWorkBlock && (panelDetail.data.isStreaming ?? false);
  const isReasoningStreaming = isReasoning && detail?.title === 'Thinking...';
  const isLiveStreaming = isWorkBlockStreaming || isReasoningStreaming;

  const title = isWorkBlock ? panelDetail.data.title || 'Activity' : detail?.title || 'Details';

  const showAgentBadge =
    isWorkBlock &&
    panelDetail.data.showAgentBadge !== false &&
    panelDetail.data.agentName &&
    panelDetail.data.agentName !== 'Goose' &&
    panelDetail.data.agentName !== 'Goose Agent';

  // Extract compact activity entries from messages
  const activityEntries = useMemo(() => {
    if (!isWorkBlock || !panelDetail.data.messages) return [];
    return extractActivityEntries(panelDetail.data.messages, isWorkBlockStreaming);
  }, [isWorkBlock, panelDetail, isWorkBlockStreaming]);

  // Auto-scroll during streaming
  useEffect(() => {
    if (!isLiveStreaming || !bottomRef.current) return;

    const scroll = () => {
      bottomRef.current?.scrollIntoView({ behavior: 'auto' });
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
              {showAgentBadge && (
                <Badge variant="default" size="sm" className="ml-1">
                  {panelDetail.data.agentName}
                  {panelDetail.data.modeName && panelDetail.data.modeName !== 'default'
                    ? ` / ${panelDetail.data.modeName}`
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
            {isWorkBlock && panelDetail.data.messages ? (
              <div className="px-3 py-2 space-y-0.5">
                {/* Status bar */}
                {panelDetail.data.toolCount > 0 && (
                  <div className="flex items-center gap-2 text-xs text-text-muted px-2 py-1.5 mb-1 border-b border-border-default">
                    <StatusDot status={isWorkBlockStreaming ? 'active' : 'completed'} />
                    <span>
                      {panelDetail.data.toolCount} tool
                      {panelDetail.data.toolCount !== 1 ? 's' : ''} used
                    </span>
                  </div>
                )}
                {activityEntries.map((entry) =>
                  entry.kind === 'thinking' ? (
                    <div
                      key={entry.id}
                      className="px-2 py-1 text-xs text-text-muted/70 italic truncate"
                      title={entry.description}
                    >
                      {entry.description}
                    </div>
                  ) : (
                    <ActivityStep
                      key={entry.id}
                      description={entry.description}
                      toolName={entry.toolName}
                      isActive={entry.isActive}
                    />
                  )
                )}
                <div ref={bottomRef} />
              </div>
            ) : isReasoning && detail ? (
              <div className="text-sm text-text-muted/90 px-4 py-4">
                <MarkdownContent content={detail.content} />
                <div ref={bottomRef} />
              </div>
            ) : null}
          </ScrollArea>
        </>
      )}
    </div>
  );
}
