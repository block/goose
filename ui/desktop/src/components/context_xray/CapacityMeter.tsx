import { useIntl, defineMessages } from '../../i18n';
import { cn } from '../../utils';
import type { ContextReport } from '../../types/contextReport';
import { Tooltip, TooltipTrigger, TooltipContent } from '../ui/Tooltip';
import { categoryColorClass, categoryMessages, commonMessages } from './categories';
import { formatTokenCount, formatPercentOf } from './format';

const i18n = defineMessages({
  tooltipDetail: {
    id: 'contextXray.meter.tooltipDetail',
    defaultMessage: '{tokens} tokens · {percent} of window',
  },
});

const USED_PORTION_MIN_WIDTH_PX = 16;

interface MeterSegmentProps {
  name: string;
  tokenCount: number;
  contextLimit: number;
  widthPercent: number;
  colorClass: string;
  roundedLeft?: boolean;
}

function MeterSegment({
  name,
  tokenCount,
  contextLimit,
  widthPercent,
  colorClass,
  roundedLeft,
}: MeterSegmentProps) {
  const intl = useIntl();
  const percent = formatPercentOf(tokenCount, contextLimit);
  const tokens = formatTokenCount(tokenCount);
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <div
          role="img"
          aria-label={`${name}: ${intl.formatMessage(i18n.tooltipDetail, { tokens, percent })}`}
          className={cn('h-full', colorClass, roundedLeft && 'rounded-l-[4px]')}
          style={{ width: `${widthPercent}%`, minWidth: 1 }}
        />
      </TooltipTrigger>
      <TooltipContent side="top">
        <div className="font-medium">{name}</div>
        <div>{intl.formatMessage(i18n.tooltipDetail, { tokens, percent })}</div>
      </TooltipContent>
    </Tooltip>
  );
}

export function CapacityMeter({ report }: { report: ContextReport }) {
  const intl = useIntl();
  const contextLimit = report.model.contextLimit;
  const visibleSegments = report.segments.filter((segment) => segment.tokenCount > 0);
  const segmentTotal = report.segments.reduce((sum, segment) => sum + segment.tokenCount, 0);
  const overheadTokens = Math.max(0, report.wireTotalTokens - segmentTotal);
  const usedTokens = Math.max(report.wireTotalTokens, segmentTotal);
  const usedPercent = contextLimit > 0 ? Math.min(100, (usedTokens / contextLimit) * 100) : 0;
  const shareOfUsed = (tokenCount: number) =>
    usedTokens > 0 ? (tokenCount / usedTokens) * 100 : 0;

  return (
    <div className="flex h-3 w-full min-w-0 gap-[2px]">
      {usedTokens > 0 && (
        <div
          data-testid="xray-meter-used"
          className="flex h-full shrink-0 gap-[2px]"
          style={{ width: `${usedPercent}%`, minWidth: USED_PORTION_MIN_WIDTH_PX }}
        >
          {visibleSegments.map((segment, index) => (
            <MeterSegment
              key={`${segment.category}-${segment.label}-${index}`}
              name={intl.formatMessage(categoryMessages[segment.category])}
              tokenCount={segment.tokenCount}
              contextLimit={contextLimit}
              widthPercent={shareOfUsed(segment.tokenCount)}
              colorClass={categoryColorClass[segment.category]}
              roundedLeft={index === 0}
            />
          ))}
          {overheadTokens > 0 && (
            <MeterSegment
              name={intl.formatMessage(commonMessages.tokenizerOverhead)}
              tokenCount={overheadTokens}
              contextLimit={contextLimit}
              widthPercent={shareOfUsed(overheadTokens)}
              colorClass="bg-border-primary"
              roundedLeft={visibleSegments.length === 0}
            />
          )}
        </div>
      )}
      <div className="h-full min-w-0 flex-1 rounded-r-[4px] bg-background-tertiary" aria-hidden="true" />
    </div>
  );
}
