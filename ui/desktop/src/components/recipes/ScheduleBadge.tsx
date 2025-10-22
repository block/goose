import { Clock, Pause, Play as PlayIcon } from 'lucide-react';
import { ScheduledJob } from '../../schedule';
import { getNextRunTime, formatRelativeTime } from '../../utils/recipeScheduleUtils';
import { cn } from '../../utils';

interface ScheduleBadgeProps {
  schedule: ScheduledJob;
  className?: string;
  showNextRun?: boolean;
}

export function ScheduleBadge({ schedule, className, showNextRun = true }: ScheduleBadgeProps) {
  const nextRun = getNextRunTime(schedule);
  const lastRun = formatRelativeTime(schedule.last_run);
  
  if (schedule.paused) {
    return (
      <div className={cn('flex items-center gap-2', className)}>
        <div className="flex items-center gap-1 px-2 py-1 rounded-md bg-yellow-500/10 text-yellow-600 dark:text-yellow-400 text-xs font-medium">
          <Pause className="w-3 h-3" />
          <span>Paused</span>
        </div>
        {lastRun && (
          <span className="text-xs text-text-muted">
            Last run: {lastRun}
          </span>
        )}
      </div>
    );
  }
  
  if (schedule.currently_running) {
    return (
      <div className={cn('flex items-center gap-2', className)}>
        <div className="flex items-center gap-1 px-2 py-1 rounded-md bg-green-500/10 text-green-600 dark:text-green-400 text-xs font-medium">
          <PlayIcon className="w-3 h-3 animate-pulse" />
          <span>Running</span>
        </div>
      </div>
    );
  }
  
  return (
    <div className={cn('flex items-center gap-2', className)}>
      <div className="flex items-center gap-1 px-2 py-1 rounded-md bg-blue-500/10 text-blue-600 dark:text-blue-400 text-xs font-medium">
        <Clock className="w-3 h-3" />
        <span>Scheduled</span>
      </div>
      {showNextRun && nextRun && (
        <span className="text-xs text-text-muted">
          {nextRun}
        </span>
      )}
    </div>
  );
}

interface ScheduleInfoProps {
  schedule: ScheduledJob;
  className?: string;
}

export function ScheduleInfo({ schedule, className }: ScheduleInfoProps) {
  const nextRun = getNextRunTime(schedule);
  const lastRun = formatRelativeTime(schedule.last_run);
  
  return (
    <div className={cn('flex flex-col gap-1 text-xs', className)}>
      <div className="flex items-center gap-2">
        <span className="text-text-muted">Schedule ID:</span>
        <span className="font-mono text-text-default">{schedule.id}</span>
      </div>
      
      {nextRun && !schedule.paused && (
        <div className="flex items-center gap-2">
          <span className="text-text-muted">Next run:</span>
          <span className="text-text-default">{nextRun}</span>
        </div>
      )}
      
      {lastRun && (
        <div className="flex items-center gap-2">
          <span className="text-text-muted">Last run:</span>
          <span className="text-text-default">{lastRun}</span>
        </div>
      )}
      
      <div className="flex items-center gap-2">
        <span className="text-text-muted">Mode:</span>
        <span className="text-text-default capitalize">
          {schedule.execution_mode || 'background'}
        </span>
      </div>
    </div>
  );
}
