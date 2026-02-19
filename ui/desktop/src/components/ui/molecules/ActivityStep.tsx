import { Check, Eye, FileText, Image, Loader2, Monitor, Terminal, Wrench } from 'lucide-react';
import type { ReactNode } from 'react';
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

// ── Props ───────────────────────────────────────────────────────────

export interface ActivityStepProps {
  /** Human-readable description of the step, e.g. "reading src/App.tsx" */
  description: string;
  /** Full qualified tool name, e.g. "developer__text_editor" */
  toolName?: string;
  /** Whether this step is currently in progress */
  isActive?: boolean;
  /** Optional CSS class */
  className?: string;
}

// ── Component ───────────────────────────────────────────────────────

export function ActivityStep({
  description,
  toolName,
  isActive = false,
  className,
}: ActivityStepProps) {
  const icon = toolName ? getToolIcon(toolName) : <Wrench size={14} />;

  return (
    <div
      className={cn(
        'flex items-center gap-2 px-2 py-1.5 rounded-md text-xs',
        isActive
          ? 'bg-blue-500/5 text-text-default'
          : 'text-text-muted hover:bg-background-muted/50',
        className
      )}
    >
      <span className={cn('shrink-0', isActive ? 'text-blue-400' : 'text-text-muted/60')}>
        {icon}
      </span>
      <span className="truncate flex-1 min-w-0">{description}</span>
      <span className="shrink-0">
        {isActive ? (
          <Loader2 size={12} className="animate-spin text-blue-400" />
        ) : (
          <Check size={12} className="text-emerald-500" />
        )}
      </span>
    </div>
  );
}
