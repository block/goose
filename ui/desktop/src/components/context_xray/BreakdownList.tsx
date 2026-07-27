import { useState } from 'react';
import { ChevronRight } from 'lucide-react';
import { useIntl, defineMessages } from '../../i18n';
import { cn } from '../../utils';
import type { ContextCategory, ContextReport, ContextSegment } from '../../types/contextReport';
import { Collapsible, CollapsibleTrigger, CollapsibleContent } from '../ui/collapsible';
import { categoryColorClass, categoryMessages } from './categories';
import { formatTokenCount, formatPercentOf } from './format';

const i18n = defineMessages({
  free: {
    id: 'contextXray.free',
    defaultMessage: 'Free',
  },
  toolCount: {
    id: 'contextXray.toolCount',
    defaultMessage: '{count, plural, one {# tool} other {# tools}}',
  },
  summaryStructured: {
    id: 'contextXray.summaryStructured',
    defaultMessage: 'structured',
  },
  summaryRawFallback: {
    id: 'contextXray.summaryRawFallback',
    defaultMessage: 'unstructured model output',
  },
});

function ContentPreview({ text }: { text: string }) {
  return (
    <pre className="text-xs font-mono text-text-secondary bg-background-secondary rounded-md p-2 max-h-48 overflow-auto whitespace-pre-wrap">
      {text}
    </pre>
  );
}

function DisclosureIcon({ open, visible }: { open: boolean; visible: boolean }) {
  if (!visible) return <span className="w-3 shrink-0" />;

  return (
    <ChevronRight
      className={cn('size-3 shrink-0 text-text-tertiary transition-transform', open && 'rotate-90')}
    />
  );
}

function TokenCount({ tokenCount }: { tokenCount: number }) {
  return (
    <span className="ml-auto font-mono text-xs text-text-primary/70 shrink-0">
      {formatTokenCount(tokenCount)}
    </span>
  );
}

interface DetailLineProps {
  label: string;
  source?: string | null;
  tokenCount: number;
  open?: boolean;
  expandable?: boolean;
}

function DetailLine({
  label,
  source,
  tokenCount,
  open = false,
  expandable = false,
}: DetailLineProps) {
  return (
    <div className="flex w-full items-center gap-2">
      <DisclosureIcon open={open} visible={expandable} />
      <span className="text-xs text-text-primary truncate">{label}</span>
      {source && <span className="text-xs text-text-tertiary truncate">{source}</span>}
      <TokenCount tokenCount={tokenCount} />
    </div>
  );
}

interface PartRowProps {
  label: string;
  source?: string | null;
  tokenCount: number;
  contentPreview?: string | null;
}

function PartRow({ label, source, tokenCount, contentPreview }: PartRowProps) {
  const [previewOpen, setPreviewOpen] = useState(false);
  const row = (
    <div className="py-0.5">
      <DetailLine
        label={label}
        source={source}
        tokenCount={tokenCount}
        open={previewOpen}
        expandable={!!contentPreview}
      />
    </div>
  );

  if (!contentPreview) return row;

  return (
    <Collapsible open={previewOpen} onOpenChange={setPreviewOpen}>
      <CollapsibleTrigger asChild>
        <button
          type="button"
          className="w-full cursor-pointer rounded-md px-1 -mx-1 text-left hover:bg-background-secondary"
        >
          {row}
        </button>
      </CollapsibleTrigger>
      <CollapsibleContent>
        <div className="py-1 pl-5">
          <ContentPreview text={contentPreview} />
        </div>
      </CollapsibleContent>
    </Collapsible>
  );
}

interface SegmentRowProps {
  segment: ContextSegment;
  categoryTotal: number;
  colorClass: string;
}

function SegmentRow({ segment, categoryTotal, colorClass }: SegmentRowProps) {
  const intl = useIntl();
  const [open, setOpen] = useState(false);
  const parts = segment.parts ?? [];
  const expandable = parts.length > 0 || !!segment.contentPreview;
  const sourceText =
    segment.category === 'tool_definitions'
      ? intl.formatMessage(i18n.toolCount, { count: parts.length })
      : segment.category === 'compaction_summary'
        ? intl.formatMessage(
            segment.source === 'structured' ? i18n.summaryStructured : i18n.summaryRawFallback
          )
        : segment.source;

  const row = (
    <div className="w-full min-w-0">
      <DetailLine
        label={segment.label}
        source={sourceText}
        tokenCount={segment.tokenCount}
        open={open}
        expandable={expandable}
      />
      <div className="mt-1 ml-5 h-1 rounded-full bg-background-tertiary">
        <div
          className={cn('h-full rounded-full', colorClass)}
          style={{
            width:
              categoryTotal > 0
                ? `${Math.min(100, (segment.tokenCount / categoryTotal) * 100)}%`
                : 0,
          }}
        />
      </div>
    </div>
  );

  if (!expandable) return <div className="py-1 px-1 -mx-1">{row}</div>;

  return (
    <Collapsible open={open} onOpenChange={setOpen}>
      <CollapsibleTrigger asChild>
        <button
          type="button"
          className="w-full cursor-pointer rounded-md py-1 px-1 -mx-1 text-left hover:bg-background-secondary"
        >
          {row}
        </button>
      </CollapsibleTrigger>
      <CollapsibleContent>
        <div className="flex flex-col py-1 pl-5">
          {parts.map((part, index) => (
            <PartRow
              key={`${part.label}-${index}`}
              label={part.label}
              source={part.source}
              tokenCount={part.tokenCount}
              contentPreview={part.contentPreview}
            />
          ))}
          {parts.length === 0 && segment.contentPreview && (
            <ContentPreview text={segment.contentPreview} />
          )}
        </div>
      </CollapsibleContent>
    </Collapsible>
  );
}

interface LegendRowShellProps {
  swatchClass: string;
  name: string;
  nameClass?: string;
  tokenCount: number;
  contextLimit: number;
}

function StaticLegendRow({
  swatchClass,
  name,
  nameClass,
  tokenCount,
  contextLimit,
}: LegendRowShellProps) {
  return (
    <div className="flex items-center gap-2 py-1.5 px-1 -mx-1">
      <span className={cn('h-2.5 w-2.5 rounded-full shrink-0', swatchClass)} />
      <span
        className={cn(
          'min-w-0 flex-1 truncate text-left text-sm',
          nameClass ?? 'text-text-primary'
        )}
      >
        {name}
      </span>
      <span className="font-mono text-xs text-text-primary/70">{formatTokenCount(tokenCount)}</span>
      <span className="w-10 text-right font-mono text-xs text-text-tertiary">
        {formatPercentOf(tokenCount, contextLimit)}
      </span>
      <span className="w-3 shrink-0" />
    </div>
  );
}

interface CategoryRowProps {
  category: ContextCategory;
  segments: ContextSegment[];
  contextLimit: number;
}

function CategoryRow({ category, segments, contextLimit }: CategoryRowProps) {
  const intl = useIntl();
  const [open, setOpen] = useState(false);
  const total = segments.reduce((sum, segment) => sum + segment.tokenCount, 0);
  const colorClass = categoryColorClass[category];

  return (
    <Collapsible open={open} onOpenChange={setOpen}>
      <CollapsibleTrigger asChild>
        <button
          type="button"
          className="flex w-full cursor-pointer items-center gap-2 rounded-md py-1.5 px-1 -mx-1 text-left hover:bg-background-secondary"
        >
          <span className={cn('h-2.5 w-2.5 rounded-full shrink-0', colorClass)} />
          <span className="min-w-0 flex-1 truncate text-sm text-text-primary">
            {intl.formatMessage(categoryMessages[category])}
          </span>
          <span className="font-mono text-xs text-text-primary/70">{formatTokenCount(total)}</span>
          <span className="w-10 text-right font-mono text-xs text-text-tertiary">
            {formatPercentOf(total, contextLimit)}
          </span>
          <ChevronRight
            className={cn(
              'size-3 shrink-0 text-text-tertiary transition-transform',
              open && 'rotate-90'
            )}
          />
        </button>
      </CollapsibleTrigger>
      <CollapsibleContent>
        <div className="flex flex-col gap-1 pb-2 pl-[18px]">
          {segments.map((segment, index) => (
            <SegmentRow
              key={`${segment.label}-${index}`}
              segment={segment}
              categoryTotal={total}
              colorClass={colorClass}
            />
          ))}
        </div>
      </CollapsibleContent>
    </Collapsible>
  );
}

export function BreakdownList({ report }: { report: ContextReport }) {
  const intl = useIntl();
  const contextLimit = report.model.contextLimit;
  const freeTokens = Math.max(0, contextLimit - report.wireTotalTokens);

  const grouped: { category: ContextCategory; segments: ContextSegment[] }[] = [];
  for (const segment of report.segments) {
    const group = grouped.find((entry) => entry.category === segment.category);
    if (group) {
      group.segments.push(segment);
    } else {
      grouped.push({ category: segment.category, segments: [segment] });
    }
  }

  return (
    <div className="flex flex-col">
      {grouped.map((group) => (
        <CategoryRow
          key={group.category}
          category={group.category}
          segments={group.segments}
          contextLimit={contextLimit}
        />
      ))}
      <StaticLegendRow
        swatchClass="border border-border-primary bg-transparent"
        name={intl.formatMessage(i18n.free)}
        nameClass="text-text-secondary"
        tokenCount={freeTokens}
        contextLimit={contextLimit}
      />
    </div>
  );
}
