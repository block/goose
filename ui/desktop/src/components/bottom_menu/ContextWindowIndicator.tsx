import BottomMenuAlertPopover from './BottomMenuAlertPopover';
import { Alert } from '../alerts';

interface ContextWindowIndicatorProps {
  totalTokens: number;
  tokenLimit: number;
  alerts: Alert[];
}

const formatTokenCount = (count: number): string => {
  if (count >= 1_000_000) return `${Math.round(count / 1_000_000)}M`;
  if (count >= 1_000) return `${Math.round(count / 1_000)}k`;
  return count.toString();
};

const getProgressColor = (percentage: number): string => {
  if (percentage <= 75) return 'text-text-primary/70';
  if (percentage <= 90) return 'text-orange-500';
  return 'text-red-500';
};

export function ContextWindowIndicator({
  totalTokens,
  tokenLimit,
  alerts,
}: ContextWindowIndicatorProps) {
  if (!tokenLimit) return null;

  const percentage = Math.round((totalTokens / tokenLimit) * 100);
  const colorClass = getProgressColor(percentage);

  return (
    <div className="flex items-center h-full">
      <BottomMenuAlertPopover alerts={alerts}>
        <span className={`text-xs font-mono ${colorClass}`}>
          {formatTokenCount(totalTokens)} / {formatTokenCount(tokenLimit)}
        </span>
      </BottomMenuAlertPopover>
    </div>
  );
}
