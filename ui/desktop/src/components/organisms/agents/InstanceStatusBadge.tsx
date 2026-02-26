import { Activity, Ban, CheckCircle, XCircle } from 'lucide-react';
import type React from 'react';
import type { InstanceStatus } from '@/lib/instances';

const statusConfig: Record<
  InstanceStatus,
  { label: string; icon: React.ElementType; dot: string; bg: string; text: string }
> = {
  running: {
    label: 'Running',
    icon: Activity,
    dot: 'bg-cyan-400 animate-pulse',
    bg: 'bg-cyan-500/10 dark:bg-cyan-400/10',
    text: 'text-cyan-700 dark:text-cyan-300',
  },
  completed: {
    label: 'Completed',
    icon: CheckCircle,
    dot: 'bg-emerald-400',
    bg: 'bg-emerald-500/10 dark:bg-emerald-400/10',
    text: 'text-emerald-700 dark:text-emerald-300',
  },
  failed: {
    label: 'Failed',
    icon: XCircle,
    dot: 'bg-red-400',
    bg: 'bg-red-500/10 dark:bg-red-400/10',
    text: 'text-red-700 dark:text-red-300',
  },
  cancelled: {
    label: 'Cancelled',
    icon: Ban,
    dot: 'bg-amber-400',
    bg: 'bg-amber-500/10 dark:bg-amber-400/10',
    text: 'text-amber-700 dark:text-amber-300',
  },
};

interface InstanceStatusBadgeProps {
  status: InstanceStatus;
  size?: 'sm' | 'md';
}

export function InstanceStatusBadge({ status, size = 'md' }: InstanceStatusBadgeProps) {
  const config = statusConfig[status] || statusConfig.failed;
  const Icon = config.icon;

  const sizeClasses = size === 'sm' ? 'text-xs px-1.5 py-0.5 gap-1' : 'text-xs px-2 py-1 gap-1.5';
  const iconSize = size === 'sm' ? 10 : 12;

  return (
    <span
      className={`inline-flex items-center rounded-full font-medium ${sizeClasses} ${config.bg} ${config.text}`}
    >
      <span className={`w-1.5 h-1.5 rounded-full ${config.dot}`} />
      <Icon size={iconSize} />
      {config.label}
    </span>
  );
}
