import { X, Sparkles } from 'lucide-react';
import type { ReactNode } from 'react';
import { cn } from '../../utils';

type ComposerChipTone = 'file' | 'agent' | 'skill' | 'automation';

const toneClasses: Record<ComposerChipTone, string> = {
  file: 'bg-background-secondary text-text-secondary hover:bg-background-secondary/80',
  agent: 'bg-blue-500/10 text-blue-700 dark:text-blue-200',
  skill: 'bg-amber-500/15 text-amber-800 dark:text-amber-100',
  automation: 'bg-green-500/10 text-green-700 dark:text-green-200',
};

interface ComposerChipProps {
  tone: ComposerChipTone;
  label: string;
  removeLabel: string;
  onRemove: () => void;
  leading?: ReactNode;
  title?: string;
  className?: string;
}

export function ComposerChip({
  tone,
  label,
  removeLabel,
  onRemove,
  leading,
  title,
  className,
}: ComposerChipProps) {
  return (
    <span
      className={cn(
        'group inline-flex h-6 max-w-64 items-center gap-1.5 rounded-full pl-2 pr-2 text-xs font-normal transition-colors',
        toneClasses[tone],
        className
      )}
      title={title ?? label}
    >
      <button
        type="button"
        onClick={onRemove}
        className="relative flex size-3.5 shrink-0 items-center justify-center rounded-full text-current focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-blue-400"
        aria-label={removeLabel}
      >
        {leading ? (
          <span className="flex items-center justify-center opacity-100 transition-opacity group-hover:opacity-0">
            {leading}
          </span>
        ) : (
          <Sparkles className="size-3 opacity-100 transition-opacity group-hover:opacity-0" />
        )}
        <X className="absolute size-3.5 opacity-0 transition-opacity group-hover:opacity-100" />
      </button>
      <span className="min-w-0 truncate">{label}</span>
    </span>
  );
}
