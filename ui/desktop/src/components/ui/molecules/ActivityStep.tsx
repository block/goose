import {
  AlertTriangle,
  Check,
  ChevronRight,
  Eye,
  FileText,
  Image,
  Loader2,
  Monitor,
  Terminal,
  Wrench,
} from 'lucide-react';
import { type ReactNode, useState } from 'react';
import { cn } from '../../../utils';

// ── Tool icon mapping ───────────────────────────────────────────────

function getToolIcon(toolName: string): ReactNode {
  const name = toolName.split('__').pop() || toolName;
  switch (name) {
    case 'shell':
      return <Terminal size={14} />;
    case 'text_editor':
      return <FileText size={14} />;
    case 'analyze':
      return <Eye size={14} />;
    case 'image_processor':
      return <Image size={14} />;
    case 'screen_capture':
      return <Monitor size={14} />;
    default:
      return <Wrench size={14} />;
  }
}

// ── Truncation helper ───────────────────────────────────────────────

function truncate(text: string, max: number): string {
  if (text.length <= max) return text;
  return `${text.slice(0, max - 1)}…`;
}

// ── Props ───────────────────────────────────────────────────────────

export interface ActivityStepProps {
  description: string;
  toolName?: string;
  isActive?: boolean;
  className?: string;
  /** Tool call arguments — shown when expanded */
  toolArgs?: Record<string, unknown>;
  /** Tool result text — shown when expanded */
  toolResult?: string;
  /** Whether the tool call resulted in an error */
  isError?: boolean;
  /** Error message if the tool call failed */
  errorMessage?: string;
}

// ── Component ───────────────────────────────────────────────────────

export function ActivityStep({
  description,
  toolName,
  isActive = false,
  className,
  toolArgs,
  toolResult,
  isError = false,
  errorMessage,
}: ActivityStepProps) {
  const [isExpanded, setIsExpanded] = useState(false);
  const icon = toolName ? getToolIcon(toolName) : <Wrench size={14} />;
  const hasDetail = !!(toolArgs || toolResult || errorMessage);
  const canExpand = hasDetail && !isActive;

  return (
    <div className={cn('rounded-md', className)}>
      {/* Collapsed row */}
      <button
        type="button"
        onClick={canExpand ? () => setIsExpanded((v) => !v) : undefined}
        className={cn(
          'flex items-center gap-2 px-2 py-1.5 w-full text-left text-xs rounded-md transition-colors',
          isActive
            ? 'bg-blue-500/5 text-text-default'
            : isError
              ? 'text-red-400 hover:bg-red-500/5'
              : 'text-text-muted hover:bg-background-muted/50',
          canExpand && 'cursor-pointer'
        )}
      >
        {/* Expand chevron */}
        {canExpand ? (
          <ChevronRight
            size={12}
            className={cn(
              'shrink-0 transition-transform text-text-muted/50',
              isExpanded && 'rotate-90'
            )}
          />
        ) : (
          <span className="w-3 shrink-0" />
        )}

        {/* Tool icon */}
        <span
          className={cn(
            'shrink-0',
            isActive ? 'text-blue-400' : isError ? 'text-red-400' : 'text-text-muted/60'
          )}
        >
          {icon}
        </span>

        {/* Description */}
        <span className="truncate flex-1 min-w-0">{description}</span>

        {/* Status indicator */}
        <span className="shrink-0">
          {isActive ? (
            <Loader2 size={12} className="animate-spin text-blue-400" />
          ) : isError ? (
            <AlertTriangle size={12} className="text-red-400" />
          ) : (
            <Check size={12} className="text-emerald-500" />
          )}
        </span>
      </button>

      {/* Expanded detail */}
      {isExpanded && canExpand && (
        <div className="ml-7 mr-2 mb-2 mt-0.5 text-xs border-l-2 border-border-default pl-3 space-y-2">
          {/* Arguments */}
          {toolArgs && Object.keys(toolArgs).length > 0 && (
            <ToolArgsView args={toolArgs} />
          )}

          {/* Error */}
          {isError && errorMessage && (
            <div className="bg-red-500/5 border border-red-500/20 rounded px-2 py-1.5">
              <span className="font-medium text-red-400">Error: </span>
              <span className="text-red-300 whitespace-pre-wrap break-all">
                {truncate(errorMessage, 2000)}
              </span>
            </div>
          )}

          {/* Result */}
          {toolResult && !isError && (
            <ToolResultView result={toolResult} />
          )}
        </div>
      )}
    </div>
  );
}

// ── Tool Arguments View ─────────────────────────────────────────────

function ToolArgsView({ args }: { args: Record<string, unknown> }) {
  return (
    <div className="space-y-1">
      <span className="text-text-muted/50 uppercase tracking-wider text-[10px]">Arguments</span>
      {Object.entries(args).map(([key, value]) => (
        <div key={key} className="flex gap-2">
          <span className="text-text-muted/70 shrink-0 min-w-[80px]">{key}</span>
          <ArgValue value={value} />
        </div>
      ))}
    </div>
  );
}

function StringArgValue({ value }: { value: string }) {
  const [expanded, setExpanded] = useState(false);
  const isLong = value.length > 120;

  if (!isLong) {
    return <span className="text-text-muted break-all">{value}</span>;
  }

  return (
    <span className="text-text-muted break-all">
      {expanded ? value : truncate(value, 120)}
      <button
        type="button"
        onClick={() => setExpanded((v) => !v)}
        className="ml-1 text-blue-400 hover:underline"
      >
        {expanded ? 'less' : 'more'}
      </button>
    </span>
  );
}

function ArgValue({ value }: { value: unknown }) {
  if (typeof value === 'string') {
    return <StringArgValue value={value} />;
  }

  if (value === null || value === undefined) {
    return <span className="text-text-muted/50 italic">null</span>;
  }

  if (typeof value === 'boolean' || typeof value === 'number') {
    return <span className="text-text-muted">{String(value)}</span>;
  }

  return (
    <pre className="text-text-muted whitespace-pre-wrap break-all overflow-x-auto max-h-[200px] overflow-y-auto">
      {truncate(JSON.stringify(value, null, 2), 2000)}
    </pre>
  );
}

// ── Tool Result View ────────────────────────────────────────────────

function ToolResultView({ result }: { result: string }) {
  const [expanded, setExpanded] = useState(false);
  const isLong = result.length > 300;
  const displayText = expanded ? result : truncate(result, 300);

  return (
    <div className="space-y-1">
      <span className="text-text-muted/50 uppercase tracking-wider text-[10px]">Result</span>
      <pre className="text-text-muted bg-background-muted/30 rounded px-2 py-1.5 whitespace-pre-wrap break-all overflow-x-auto max-h-[300px] overflow-y-auto">
        {displayText}
      </pre>
      {isLong && (
        <button
          type="button"
          onClick={() => setExpanded((v) => !v)}
          className="text-blue-400 hover:underline text-[10px]"
        >
          {expanded ? 'Show less' : `Show all (${result.length} chars)`}
        </button>
      )}
    </div>
  );
}

// ── Thinking Entry ──────────────────────────────────────────────────

export interface ThinkingEntryProps {
  text: string;
  className?: string;
}

export function ThinkingEntry({ text, className }: ThinkingEntryProps) {
  return (
    <div
      className={cn(
        'px-2 py-1.5 text-xs text-text-muted/70 italic leading-relaxed',
        className
      )}
    >
      {text}
    </div>
  );
}
