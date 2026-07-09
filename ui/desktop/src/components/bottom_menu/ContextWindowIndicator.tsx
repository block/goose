import { defineMessages, useIntl } from '../../i18n';
import { formatTokenCount } from '../../utils/usageFormatting';

interface ContextWindowIndicatorProps {
  totalTokens: number;
  tokenLimit: number;
  onOpenXray?: () => void;
}

const i18n = defineMessages({
  openXray: {
    id: 'contextWindowIndicator.openXray',
    defaultMessage: 'Open context x-ray',
  },
});

const getProgressColor = (percentage: number, interactive: boolean): string => {
  if (percentage <= 75)
    return interactive ? 'text-text-primary/70 hover:text-text-primary' : 'text-text-primary/70';
  if (percentage <= 90)
    return interactive ? 'text-orange-500 hover:text-orange-400' : 'text-orange-500';
  return interactive ? 'text-red-500 hover:text-red-400' : 'text-red-500';
};

const getDotColor = (percentage: number): string => {
  if (percentage <= 75) return 'bg-[#00b300]';
  if (percentage <= 90) return 'bg-orange-500';
  return 'bg-red-500';
};

export function ContextWindowIndicator({
  totalTokens,
  tokenLimit,
  onOpenXray,
}: ContextWindowIndicatorProps) {
  const intl = useIntl();
  if (!tokenLimit) return null;

  const percentage = Math.round((totalTokens / tokenLimit) * 100);
  const colorClass = getProgressColor(percentage, !!onOpenXray);
  const text = `${formatTokenCount(totalTokens)} / ${formatTokenCount(tokenLimit)}`;
  const content = (
    <>
      <span className={`size-1.5 rounded-full ${getDotColor(percentage)}`} aria-hidden="true" />
      <span className={`text-xs font-mono transition-colors ${colorClass}`}>{text}</span>
    </>
  );

  if (!onOpenXray) {
    return <div className="flex min-h-5 items-center gap-1.5 px-1">{content}</div>;
  }

  return (
    <div className="flex items-center h-full">
      <button
        type="button"
        aria-label={intl.formatMessage(i18n.openXray)}
        className="flex min-h-5 cursor-pointer items-center gap-1.5 rounded px-1 hover:bg-background-secondary"
        onClick={onOpenXray}
      >
        {content}
      </button>
    </div>
  );
}
