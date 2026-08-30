import { useCallback, useEffect, useMemo, useState } from 'react';
import { defineMessages, useIntl } from '../i18n';
import {
  acpGetSessionActivity,
  type SessionActivity,
  type SessionActivityDay,
  type SessionActivityModel,
} from '../acp/sessionActivity';
import { formatTokenCount } from '../utils/usageFormatting';
import {
  ACTIVITY_WEEKDAYS,
  activityLevel,
  buildYearHeatmap,
  heatmapWeekCount,
  type HeatmapCell,
} from '../utils/activityHeatmap';
import { Button } from './ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from './ui/dialog';
import { Tooltip, TooltipContent, TooltipTrigger } from './ui/Tooltip';
import { cn } from '../utils';
import { View, ViewOptions } from '../utils/navigationUtils';

const CELL = 10;
const GAP = 3;

const i18n = defineMessages({
  yearTokens: {
    id: 'activityHeatmap.yearTokens',
    defaultMessage: '{count} tokens this year',
  },
  yearSessions: {
    id: 'activityHeatmap.yearSessions',
    defaultMessage: '{count, plural, one {# session} other {# sessions}}',
  },
  exploreByModel: {
    id: 'activityHeatmap.exploreByModel',
    defaultMessage: 'Tokens by model',
  },
  previousYear: {
    id: 'activityHeatmap.previousYear',
    defaultMessage: 'Previous year',
  },
  nextYear: {
    id: 'activityHeatmap.nextYear',
    defaultMessage: 'Next year',
  },
  loading: {
    id: 'activityHeatmap.loading',
    defaultMessage: 'Loading activity…',
  },
  loadError: {
    id: 'activityHeatmap.loadError',
    defaultMessage: "Couldn't load activity",
  },
  empty: {
    id: 'activityHeatmap.empty',
    defaultMessage: 'No sessions this year',
  },
  dayTooltip: {
    id: 'activityHeatmap.dayTooltip',
    defaultMessage:
      '{date}: {sessions, plural, one {# session} other {# sessions}}, {tokens} tokens',
  },
  noActivity: {
    id: 'activityHeatmap.noActivity',
    defaultMessage: '{date}: no sessions',
  },
  daySessionsTitle: {
    id: 'activityHeatmap.daySessionsTitle',
    defaultMessage: 'Sessions on {date}',
  },
  daySessionsDescription: {
    id: 'activityHeatmap.daySessionsDescription',
    defaultMessage: '{sessions, plural, one {# session} other {# sessions}} · {tokens} tokens',
  },
  unnamedSession: {
    id: 'activityHeatmap.unnamedSession',
    defaultMessage: 'Untitled session',
  },
  modelDialogTitle: {
    id: 'activityHeatmap.modelDialogTitle',
    defaultMessage: 'Tokens by model in {year}',
  },
  modelDialogDescription: {
    id: 'activityHeatmap.modelDialogDescription',
    defaultMessage: '{tokens} tokens across {sessions, plural, one {# session} other {# sessions}}',
  },
  unknownModel: {
    id: 'activityHeatmap.unknownModel',
    defaultMessage: 'Unknown model',
  },
  weekday: {
    id: 'activityHeatmap.weekday',
    defaultMessage: '{day}',
  },
});

const LEVEL_CLASS: Record<0 | 1 | 2 | 3 | 4, string> = {
  0: 'fill-[color-mix(in_srgb,var(--color-text-primary)_12%,transparent)]',
  1: 'fill-green-100/50',
  2: 'fill-green-100',
  3: 'fill-green-200',
  4: 'fill-[color-mix(in_srgb,var(--color-green-200)_80%,black)]',
};

function modelLabel(model: SessionActivityModel, unknown: string): string {
  if (model.modelId && model.providerId) {
    return `${model.providerId} / ${model.modelId}`;
  }
  return model.modelId || model.providerId || unknown;
}

function sessionLabel(name: string, fallback: string): string {
  const trimmed = name.trim();
  return trimmed.length > 0 ? trimmed : fallback;
}

export default function ActivityHeatmap({
  setView,
}: {
  setView: (view: View, viewOptions?: ViewOptions) => void;
}) {
  const intl = useIntl();
  const currentYear = new Date().getUTCFullYear();
  const [year, setYear] = useState(currentYear);
  const [activity, setActivity] = useState<SessionActivity | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(false);
  const [selectedDay, setSelectedDay] = useState<SessionActivityDay | null>(null);
  const [modelsOpen, setModelsOpen] = useState(false);

  const load = useCallback(async (requestedYear: number) => {
    setLoading(true);
    setError(false);
    try {
      const next = await acpGetSessionActivity(requestedYear);
      setActivity(next);
    } catch (loadError) {
      console.error('Failed to load session activity:', loadError);
      setActivity(null);
      setError(true);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load(year);
  }, [load, year]);

  const daysByDate = useMemo(() => {
    const map = new Map<string, SessionActivityDay>();
    for (const day of activity?.days ?? []) {
      map.set(day.date, day);
    }
    return map;
  }, [activity]);

  const cells = useMemo(() => buildYearHeatmap(year), [year]);
  const weekCount = heatmapWeekCount(cells);
  const maxSessionCount = useMemo(
    () => Math.max(0, ...(activity?.days.map((day) => day.sessionCount) ?? [0])),
    [activity]
  );

  const openSession = (sessionId: string) => {
    setSelectedDay(null);
    setView('pair', {
      disableAnimation: true,
      resumeSessionId: sessionId,
    });
  };

  return (
    <div className="mb-8">
      <div className="flex items-baseline justify-between gap-3 mb-3">
        <div>
          <p className="text-sm text-text-primary">
            {intl.formatMessage(i18n.yearTokens, {
              count: formatTokenCount(activity?.totalTokens ?? 0),
            })}
          </p>
          <p className="text-xs text-text-secondary">
            {intl.formatMessage(i18n.yearSessions, { count: activity?.totalSessions ?? 0 })}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button
            type="button"
            variant="ghost"
            size="xs"
            shape="pill"
            onClick={() => setYear((current) => current - 1)}
            aria-label={intl.formatMessage(i18n.previousYear)}
          >
            ‹
          </Button>
          <span className="text-sm tabular-nums text-text-secondary w-10 text-center">{year}</span>
          <Button
            type="button"
            variant="ghost"
            size="xs"
            shape="pill"
            onClick={() => setYear((current) => Math.min(currentYear, current + 1))}
            disabled={year >= currentYear}
            aria-label={intl.formatMessage(i18n.nextYear)}
          >
            ›
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="xs"
            shape="pill"
            onClick={() => setModelsOpen(true)}
            disabled={!activity || activity.models.length === 0}
          >
            {intl.formatMessage(i18n.exploreByModel)}
          </Button>
        </div>
      </div>

      {loading ? (
        <p className="text-xs text-text-secondary">{intl.formatMessage(i18n.loading)}</p>
      ) : error ? (
        <p className="text-xs text-text-secondary">{intl.formatMessage(i18n.loadError)}</p>
      ) : (
        <>
          <HeatmapGrid
            cells={cells}
            weekCount={weekCount}
            daysByDate={daysByDate}
            maxSessionCount={maxSessionCount}
            onSelectDay={setSelectedDay}
          />
          {!activity?.days.length && (
            <p className="text-xs text-text-secondary mt-2">{intl.formatMessage(i18n.empty)}</p>
          )}
        </>
      )}

      <Dialog open={selectedDay !== null} onOpenChange={(open) => !open && setSelectedDay(null)}>
        <DialogContent className="max-w-md">
          {selectedDay && (
            <>
              <DialogHeader>
                <DialogTitle>
                  {intl.formatMessage(i18n.daySessionsTitle, { date: selectedDay.date })}
                </DialogTitle>
                <DialogDescription>
                  {intl.formatMessage(i18n.daySessionsDescription, {
                    sessions: selectedDay.sessionCount,
                    tokens: formatTokenCount(selectedDay.totalTokens),
                  })}
                </DialogDescription>
              </DialogHeader>
              <ul className="space-y-1 max-h-72 overflow-y-auto">
                {selectedDay.sessions.map((session) => (
                  <li key={session.id}>
                    <button
                      type="button"
                      className="w-full text-left px-2 py-1.5 rounded-md hover:bg-background-secondary"
                      onClick={() => openSession(session.id)}
                    >
                      <div className="text-sm text-text-primary truncate">
                        {sessionLabel(
                          session.name,
                          intl.formatMessage(i18n.unnamedSession)
                        )}
                      </div>
                      <div className="text-xs text-text-secondary">
                        {formatTokenCount(session.totalTokens)}
                        {session.modelId ? ` · ${session.modelId}` : ''}
                      </div>
                    </button>
                  </li>
                ))}
              </ul>
            </>
          )}
        </DialogContent>
      </Dialog>

      <Dialog open={modelsOpen} onOpenChange={setModelsOpen}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>
              {intl.formatMessage(i18n.modelDialogTitle, { year })}
            </DialogTitle>
            <DialogDescription>
              {intl.formatMessage(i18n.modelDialogDescription, {
                tokens: formatTokenCount(activity?.totalTokens ?? 0),
                sessions: activity?.totalSessions ?? 0,
              })}
            </DialogDescription>
          </DialogHeader>
          <ul className="space-y-2 max-h-72 overflow-y-auto">
            {(activity?.models ?? []).map((model) => (
              <li
                key={`${model.providerId ?? ''}:${model.modelId ?? ''}`}
                className="flex items-baseline justify-between gap-3 text-sm"
              >
                <span className="truncate text-text-primary">
                  {modelLabel(model, intl.formatMessage(i18n.unknownModel))}
                </span>
                <span className="tabular-nums text-text-secondary shrink-0">
                  {formatTokenCount(model.totalTokens)}
                </span>
              </li>
            ))}
          </ul>
        </DialogContent>
      </Dialog>
    </div>
  );
}

function HeatmapGrid({
  cells,
  weekCount,
  daysByDate,
  maxSessionCount,
  onSelectDay,
}: {
  cells: HeatmapCell[];
  weekCount: number;
  daysByDate: Map<string, SessionActivityDay>;
  maxSessionCount: number;
  onSelectDay: (day: SessionActivityDay) => void;
}) {
  const intl = useIntl();
  const width = weekCount * (CELL + GAP) - GAP;
  const height = 7 * (CELL + GAP) - GAP;

  return (
    <div className="flex gap-2 overflow-x-auto">
      <div className="flex flex-col justify-between py-[1px] text-[9px] leading-none text-text-secondary">
        {ACTIVITY_WEEKDAYS.map((day, index) => (
          <span key={day} className={index % 2 === 0 ? 'invisible' : undefined}>
            {intl.formatMessage(i18n.weekday, { day })}
          </span>
        ))}
      </div>
      <svg
        role="img"
        width={width}
        height={height}
        viewBox={`0 0 ${width} ${height}`}
        className="min-w-0"
      >
        {cells.map((cell) => {
          const day = daysByDate.get(cell.date);
          const level = cell.inYear
            ? activityLevel(day?.sessionCount ?? 0, maxSessionCount)
            : 0;
          const label = day
            ? intl.formatMessage(i18n.dayTooltip, {
                date: cell.date,
                sessions: day.sessionCount,
                tokens: formatTokenCount(day.totalTokens),
              })
            : intl.formatMessage(i18n.noActivity, { date: cell.date });
          const x = cell.weekIndex * (CELL + GAP);
          const y = cell.weekday * (CELL + GAP);

          return (
            <Tooltip key={cell.date}>
              <TooltipTrigger asChild>
                <rect
                  x={x}
                  y={y}
                  width={CELL}
                  height={CELL}
                  rx={2}
                  className={cn(
                    LEVEL_CLASS[level],
                    cell.inYear && day
                      ? 'cursor-pointer'
                      : cell.inYear
                        ? 'cursor-default'
                        : 'opacity-20'
                  )}
                  tabIndex={cell.inYear ? 0 : -1}
                  aria-label={label}
                  onClick={() => {
                    if (day) onSelectDay(day);
                  }}
                  onKeyDown={(event) => {
                    if (day && (event.key === 'Enter' || event.key === ' ')) {
                      event.preventDefault();
                      onSelectDay(day);
                    }
                  }}
                />
              </TooltipTrigger>
              <TooltipContent>{label}</TooltipContent>
            </Tooltip>
          );
        })}
      </svg>
    </div>
  );
}
