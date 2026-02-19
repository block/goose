import { ChevronRight } from 'lucide-react';
import { useCallback, useEffect, useMemo, useRef } from 'react';
import type { Message } from '../../api';
import { useReasoningDetail, type WorkBlockDetail } from '../../contexts/ReasoningDetailContext';
import FlyingBird from '../branding/FlyingBird';
import GooseLogo from '../branding/GooseLogo';

/** Convert snake_case to Title Case */
function snakeToTitle(s: string): string {
  return s
    .split('_')
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join(' ');
}

/** Extract the tool name (after last __) from a fully-qualified name like "developer__shell" */
function getToolName(fullName: string): string {
  const parts = fullName.split('__');
  return parts[parts.length - 1] || fullName;
}

/** Build a concise description of a tool call (e.g. "editing src/App.tsx") */
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
      // Generic fallback: "Tool Name" + first string arg value
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

/**
 * Extract the first meaningful sentence from a text block.
 * Strips markdown/HTML, splits on sentence boundaries, and truncates.
 */
function firstSentence(text: string, maxLen = 100): string {
  const clean = text
    .replace(/<[^>]*>/g, '') // strip HTML
    .replace(/```[\s\S]*?```/g, '') // strip code blocks
    .replace(/`[^`]+`/g, '') // strip inline code
    .replace(/#{1,6}\s+/g, '') // strip markdown headings
    .replace(/\*{1,2}([^*]+)\*{1,2}/g, '$1') // strip bold/italic
    .replace(/\[([^\]]+)\]\([^)]+\)/g, '$1') // strip links
    .replace(/\n+/g, ' ') // collapse newlines
    .trim();
  if (!clean) return '';

  // Split on sentence boundaries: . ! ? or — followed by space/end
  const match = clean.match(/^(.+?[.!?:—])(?:\s|$)/);
  const sentence = match ? match[1].trim() : clean;
  return sentence.length > maxLen ? `${sentence.slice(0, maxLen - 1)}…` : sentence;
}

/**
 * Extract a one-liner summarizing the current work block thinking.
 *
 * Priority:
 *   1. Latest assistant text (first sentence) — captures the LLM's *intent*
 *      (e.g. "Let me fix the render loop" / "Now I'll update the context")
 *   2. Last tool call description — mechanical fallback when no text exists
 *      (e.g. "editing src/App.tsx" / "running npm install")
 */
function extractOneLiner(messages: Message[]): string {
  // Primary: latest assistant text — the LLM's explanation of what it's doing
  for (let i = messages.length - 1; i >= 0; i--) {
    const msg = messages[i];
    if (msg.role !== 'assistant') continue;
    // Search text content items (skip toolRequest items)
    for (let j = msg.content.length - 1; j >= 0; j--) {
      const c = msg.content[j];
      if (c.type === 'text' && c.text?.trim()) {
        const sentence = firstSentence(c.text);
        if (sentence.length >= 10) return sentence;
      }
    }
  }

  // Fallback: last tool call description
  for (let i = messages.length - 1; i >= 0; i--) {
    const msg = messages[i];
    if (msg.role !== 'assistant') continue;
    for (let j = msg.content.length - 1; j >= 0; j--) {
      const c = msg.content[j];
      if (c.type === 'toolRequest') {
        const toolCall = c.toolCall as
          | { status?: string; value?: { name?: string; arguments?: Record<string, unknown> } }
          | undefined;
        if (toolCall?.status === 'success' && toolCall.value?.name) {
          const desc = describeToolCall(toolCall.value.name, toolCall.value.arguments || {});
          return desc.length > 100 ? `${desc.slice(0, 97)}…` : desc;
        }
      }
    }
  }
  return '';
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

interface WorkBlockIndicatorProps {
  messages: Message[];
  blockId: string;
  isStreaming: boolean;
  agentName?: string;
  modeName?: string;
  sessionId?: string;
  toolCallNotifications?: Map<string, unknown[]>;
}

export default function WorkBlockIndicator({
  messages,
  blockId,
  isStreaming,
  agentName,
  modeName,
  sessionId,
  toolCallNotifications,
}: WorkBlockIndicatorProps) {
  const { toggleWorkBlock, panelDetail, isOpen, updateWorkBlock, closeDetail } =
    useReasoningDetail();

  const hasAutoOpened = useRef(false);

  const oneLiner = useMemo(() => extractOneLiner(messages), [messages]);
  const toolCount = useMemo(() => countToolCalls(messages), [messages]);

  const isActive =
    isOpen && panelDetail?.type === 'workblock' && panelDetail.data.messageId === blockId;

  // Refs keep latest values accessible from effects without adding deps
  const latestRef = useRef({
    messages,
    toolCount,
    isStreaming,
    toolCallNotifications,
    isActive,
  });
  latestRef.current = { messages, toolCount, isStreaming, toolCallNotifications, isActive };

  const buildDetail = useCallback(
    (): WorkBlockDetail => ({
      title: latestRef.current.isStreaming
        ? 'Goose is working on it…'
        : `Worked on ${latestRef.current.messages.length} steps`,
      messageId: blockId,
      messages: latestRef.current.messages,
      toolCount: latestRef.current.toolCount,
      isStreaming: latestRef.current.isStreaming,
      agentName,
      modeName,
      sessionId,
      toolCallNotifications: latestRef.current.toolCallNotifications as
        | Map<string, unknown[]>
        | undefined,
    }),
    [blockId, agentName, modeName, sessionId]
  );

  const handleClick = () => {
    toggleWorkBlock(buildDetail());
  };

  // Auto-open the panel when streaming starts (once per block)
  useEffect(() => {
    if (isStreaming && messages.length > 0 && !hasAutoOpened.current) {
      hasAutoOpened.current = true;
      toggleWorkBlock(buildDetail());
    }
  }, [isStreaming, messages.length, toggleWorkBlock, buildDetail]);

  // Live-update the panel content when messages or tool count change during streaming.
  // messages.length and toolCount are value-based change detectors — they trigger
  // this effect when content meaningfully changes. buildDetail reads from latestRef.
  // biome-ignore lint/correctness/useExhaustiveDependencies: messages.length and toolCount are intentional change detectors
  useEffect(() => {
    if (isActive && isStreaming) {
      updateWorkBlock(buildDetail());
    }
  }, [messages.length, toolCount, isStreaming, isActive, updateWorkBlock, buildDetail]);

  // Auto-close the panel when streaming ends so the final answer is visible
  const prevStreamingRef = useRef(isStreaming);
  useEffect(() => {
    if (prevStreamingRef.current && !isStreaming && isActive) {
      closeDetail();
    }
    prevStreamingRef.current = isStreaming;
  }, [isStreaming, isActive, closeDetail]);

  const displayAgent = agentName || 'Goose Agent';
  const displayMode = modeName || 'assistant';

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
        {/* Animated goose icon */}
        <div className="shrink-0">
          {isStreaming ? (
            <FlyingBird className="flex-shrink-0" cycleInterval={150} />
          ) : (
            <GooseLogo size="small" hover={false} />
          )}
        </div>

        {/* Content */}
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-1 flex-wrap">
            <span className="text-xs font-medium text-text-default">
              {isStreaming ? 'Goose is working on it…' : `Worked on ${messages.length} steps`}
            </span>
            {toolCount > 0 && !isStreaming && (
              <>
                <span className="text-xs text-text-muted/50 mx-0.5">·</span>
                <span className="text-xs text-text-muted/70">
                  {toolCount} tool{toolCount !== 1 ? 's' : ''} used
                </span>
              </>
            )}
          </div>
          <div className="flex items-center gap-1 mt-0.5">
            <span className="text-xs font-medium text-blue-500">{displayAgent}</span>
            <span className="text-xs text-text-muted/40">/</span>
            <span className="text-xs font-medium text-blue-500">{displayMode}</span>
            {oneLiner && (
              <>
                <span className="text-xs text-text-muted/40 mx-0.5">·</span>
                <span className="text-xs text-text-muted/50 truncate">{oneLiner}</span>
              </>
            )}
          </div>
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
