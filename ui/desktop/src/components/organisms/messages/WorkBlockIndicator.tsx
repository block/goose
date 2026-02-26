import { ChevronRight } from 'lucide-react';
import { useCallback, useEffect, useMemo, useRef } from 'react';
import type { Message } from '@/api';
import { useReasoningDetail, type WorkBlockDetail } from '@/contexts/ReasoningDetailContext';
import FlyingBird from '@/components/atoms/branding/FlyingBird';
import GooseLogo from '@/components/atoms/branding/GooseLogo';
import { Badge } from '@/components/atoms/badge';
import { StatusDot } from '@/components/atoms/status-dot';

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

function describeToolCall(name: string, args: Record<string, unknown>): string {
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
    default: {
      const display = snakeToTitle(toolName);
      const firstStr = Object.values(args).find((v) => typeof v === 'string');
      if (firstStr) {
        const val = str(firstStr);
        return `${display}: ${val.length > 60 ? `${val.slice(0, 57)}…` : val}`;
      }
      return display;
    }
  }
  return snakeToTitle(toolName);
}

function extractLastToolDescription(messages: Message[]): string {
  // Search backwards: prefer the latest assistant thinking text over tool descriptions.
  // Thinking text (e.g. "I'll start by analyzing...") gives better context than
  // tool call descriptions (e.g. "running command...") for the one-liner.
  let lastToolDesc = '';
  for (let i = messages.length - 1; i >= 0; i--) {
    const msg = messages[i];
    if (msg.role !== 'assistant') continue;
    for (let j = msg.content.length - 1; j >= 0; j--) {
      const c = msg.content[j];
      if (c.type === 'text') {
        const text = (c as { text: string }).text?.trim();
        if (text) {
          // Extract first sentence for a clean one-liner
          const match = text.match(/^(.+?[.!?:—])\s/);
          const snippet = match ? match[1] : text;
          return snippet.length > 100 ? `${snippet.slice(0, 97)}…` : snippet;
        }
      }
      if (c.type === 'toolRequest' && !lastToolDesc) {
        const toolCall = c.toolCall as
          | { status?: string; value?: { name?: string; arguments?: Record<string, unknown> } }
          | undefined;
        if (toolCall?.value?.name) {
          const desc = describeToolCall(toolCall.value.name, toolCall.value.arguments || {});
          lastToolDesc = desc.length > 100 ? `${desc.slice(0, 97)}…` : desc;
        }
      }
    }
  }
  return lastToolDesc;
}

function countToolCalls(messages: Message[]): number {
  let count = 0;
  for (const msg of messages) {
    for (const c of msg.content) {
      if (c.type === 'toolRequest') count++;
    }
  }
  return count;
}

// ── Props ───────────────────────────────────────────────────────────

interface WorkBlockIndicatorProps {
  messages: Message[];
  blockId: string;
  isStreaming: boolean;
  agentName?: string;
  modeName?: string;
  showAgentBadge?: boolean;
  sessionId?: string;
  toolCallNotifications?: Map<string, unknown[]>;
}

// ── Component ───────────────────────────────────────────────────────

export default function WorkBlockIndicator({
  messages,
  blockId,
  isStreaming,
  agentName,
  modeName,
  showAgentBadge = true,
  sessionId,
  toolCallNotifications,
}: WorkBlockIndicatorProps) {
  const { isOpen, panelDetail, toggleWorkBlock, updateWorkBlock, closeDetail } =
    useReasoningDetail();

  const hasAutoOpened = useRef(false);
  const oneLiner = useMemo(() => extractLastToolDescription(messages), [messages]);
  const toolCount = countToolCalls(messages);

  const isActive =
    isOpen && panelDetail?.type === 'workblock' && panelDetail.data.messageId === blockId;

  // Keep a mutable ref so callbacks always read fresh values without re-creating
  const latestRef = useRef({ messages, toolCount, isStreaming, toolCallNotifications, isActive });
  latestRef.current = { messages, toolCount, isStreaming, toolCallNotifications, isActive };

  const buildDetail = useCallback(
    (): WorkBlockDetail => ({
      title: 'Activity',
      messageId: blockId,
      messages: latestRef.current.messages,
      toolCount: latestRef.current.toolCount,
      isStreaming: latestRef.current.isStreaming,
      agentName,
      modeName,
      showAgentBadge,
      sessionId,
      toolCallNotifications: latestRef.current.toolCallNotifications as
        | Map<string, unknown[]>
        | undefined,
    }),
    [blockId, agentName, modeName, showAgentBadge, sessionId]
  );

  const handleClick = () => {
    toggleWorkBlock(buildDetail());
  };

  // Auto-open on first streaming
  useEffect(() => {
    if (isStreaming && messages.length > 0 && !hasAutoOpened.current) {
      hasAutoOpened.current = true;
      toggleWorkBlock(buildDetail());
    }
  }, [isStreaming, messages.length, toggleWorkBlock, buildDetail]);

  // Live-update during streaming
  // biome-ignore lint/correctness/useExhaustiveDependencies: messages.length and toolCount are intentional change detectors
  useEffect(() => {
    if (isActive && isStreaming) {
      updateWorkBlock(buildDetail());
    }
  }, [messages.length, toolCount, isStreaming, isActive, updateWorkBlock, buildDetail]);

  // Auto-close when streaming ends
  const prevStreamingRef = useRef(isStreaming);
  useEffect(() => {
    if (prevStreamingRef.current && !isStreaming && isActive) {
      closeDetail();
    }
    prevStreamingRef.current = isStreaming;
  }, [isStreaming, isActive, closeDetail]);

  const displayAgent = agentName || 'Goose';
  const displayMode = modeName || 'default';
  const isNonDefaultAgent = !!agentName && agentName !== 'Goose' && agentName !== 'Goose Agent';

  return (
    <div className="py-1.5 px-2">
      <button
        type="button"
        onClick={handleClick}
        className={`
          flex items-center gap-2.5 px-3 py-2.5 rounded-lg w-full text-left
          transition-colors duration-150 cursor-pointer
          ${isActive ? 'bg-background-muted' : 'hover:bg-background-muted/50'}
        `}
      >
        {/* Icon */}
        <div className="shrink-0">
          {isStreaming ? (
            <FlyingBird className="flex-shrink-0" cycleInterval={150} />
          ) : (
            <GooseLogo size="small" hover={false} />
          )}
        </div>

        {/* Content */}
        <div className="flex-1 min-w-0">
          {/* Line 1: Status + metrics */}
          <div className="flex items-center gap-1.5">
            <StatusDot status={isStreaming ? 'active' : 'completed'} />
            <span className="text-xs font-medium text-text-default">
              {isStreaming ? 'Working…' : `${messages.length} steps`}
            </span>
            {toolCount > 0 && !isStreaming && (
              <>
                <span className="text-xs text-text-muted/40">·</span>
                <span className="text-xs text-text-muted/70">
                  {toolCount} tool{toolCount !== 1 ? 's' : ''}
                </span>
              </>
            )}
            {showAgentBadge && isNonDefaultAgent && (
              <Badge variant="default" size="sm">
                {displayAgent}
                {modeName && modeName !== 'default' ? ` / ${displayMode}` : ''}
              </Badge>
            )}
          </div>

          {/* Line 2: One-liner activity description */}
          {oneLiner && (
            <p className="text-xs text-text-muted/60 truncate mt-0.5 leading-snug">{oneLiner}</p>
          )}
        </div>

        {/* Chevron */}
        <ChevronRight
          size={14}
          className={`shrink-0 text-text-muted/40 transition-transform ${isActive ? 'rotate-90' : ''}`}
        />
      </button>
    </div>
  );
}
