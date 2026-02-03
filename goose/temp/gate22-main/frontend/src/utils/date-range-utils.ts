import { type DateRange } from "react-day-picker";
import { z } from "zod";
import { addMinutes } from "date-fns";
type DateTrunc = "month" | "week" | "day" | "hour" | "minute";

export const DEFAULT_DASHBOARD_AGGREGATION_SELECTION = "24 hours" as const;
export const DASHBOARD_AGGREGATION_PLACEHOLDER = "Custom" as const;

export const DASHBOARD_AGGREGATION_OPTIONS = [
  "5 min",
  "30 min",
  "1 hour",
  "3 hours",
  "24 hours",
  "3 days",
  "7 days",
  "1 month",
  "3 months",
  "1 year",
] as const;

export const TABLE_AGGREGATION_OPTIONS = [
  "30 min",
  "1 hour",
  "6 hours",
  "24 hours",
  "3 days",
  "7 days",
  "14 days",
  "1 month",
  "3 months",
  "All time",
] as const;

export type DashboardDateRangeAggregationOption = (typeof DASHBOARD_AGGREGATION_OPTIONS)[number];

export type TableDateRange = {
  from: Date;
};

export type TableDateRangeAggregationOption = (typeof TABLE_AGGREGATION_OPTIONS)[number];

export type DashboardDateRange = {
  from: Date;
  to: Date;
};

export type DateRangeAggregationOption =
  | DashboardDateRangeAggregationOption
  | TableDateRangeAggregationOption;

export type DashboardDateRangeOptions =
  | DashboardDateRangeAggregationOption
  | typeof DASHBOARD_AGGREGATION_PLACEHOLDER;

export type TableDateRangeOptions = TableDateRangeAggregationOption;
export type DashboardDateRangeAggregationSettings = Record<
  DashboardDateRangeAggregationOption,
  {
    date_trunc: DateTrunc;
    minutes: number;
  }
>;

export const dateTimeAggregationOptions = [
  ...TABLE_AGGREGATION_OPTIONS,
  ...DASHBOARD_AGGREGATION_OPTIONS,
] as const;

export const dashboardDateRangeAggregationSettings: DashboardDateRangeAggregationSettings = {
  "1 year": {
    date_trunc: "month",
    minutes: 365 * 24 * 60,
  },
  "3 months": {
    date_trunc: "week",
    minutes: 3 * 30 * 24 * 60,
  },
  "1 month": {
    date_trunc: "day",
    minutes: 30 * 24 * 60,
  },
  "7 days": {
    date_trunc: "hour",
    minutes: 7 * 24 * 60,
  },
  "3 days": {
    date_trunc: "hour",
    minutes: 3 * 24 * 60,
  },
  "24 hours": {
    date_trunc: "hour",
    minutes: 24 * 60,
  },
  "3 hours": {
    date_trunc: "minute",
    minutes: 3 * 60,
  },
  "1 hour": {
    date_trunc: "minute",
    minutes: 60,
  },
  "30 min": {
    date_trunc: "minute",
    minutes: 30,
  },
  "5 min": {
    date_trunc: "minute",
    minutes: 5,
  },
};

export const SelectedTimeOptionSchema = z
  .discriminatedUnion("filterSource", [
    z.object({
      filterSource: z.literal("TABLE"),
      option: z.enum(TABLE_AGGREGATION_OPTIONS),
    }),
    z.object({
      filterSource: z.literal("DASHBOARD"),
      option: z.enum(DASHBOARD_AGGREGATION_OPTIONS),
    }),
  ])
  .optional();

export const isDashboardDateRangeOptionAvailable = ({
  option,
  limitDays,
}: {
  option: DashboardDateRangeAggregationOption;
  limitDays: number | false;
}) => {
  if (limitDays === false) return true;

  const { minutes } = dashboardDateRangeAggregationSettings[option];
  return limitDays >= minutes / (24 * 60);
};

type SelectedTimeOption = z.infer<typeof SelectedTimeOptionSchema>;

const TABLE_DATE_RANGE_AGGREGATION_SETTINGS = new Map<
  TableDateRangeAggregationOption,
  number | null
>([
  ["3 months", 3 * 30 * 24 * 60],
  ["1 month", 30 * 24 * 60],
  ["14 days", 14 * 24 * 60],
  ["7 days", 7 * 24 * 60],
  ["3 days", 3 * 24 * 60],
  ["24 hours", 24 * 60],
  ["6 hours", 6 * 60],
  ["1 hour", 60],
  ["30 min", 30],
  ["All time", null],
]);

export const isTableDataRangeOptionAvailable = ({
  option,
  limitDays,
}: {
  option: TableDateRangeAggregationOption;
  limitDays: number | false;
}) => {
  if (limitDays === false) return true;

  const durationMinutes = TABLE_DATE_RANGE_AGGREGATION_SETTINGS.get(option);
  if (!durationMinutes) return false;

  return limitDays >= durationMinutes / (24 * 60);
};

export const getDateFromOption = (selectedTimeOption: SelectedTimeOption): Date | undefined => {
  if (!selectedTimeOption) return undefined;

  const { filterSource, option } = selectedTimeOption;
  if (filterSource === "TABLE") {
    const setting = TABLE_DATE_RANGE_AGGREGATION_SETTINGS.get(option);
    if (!setting) return undefined;

    return addMinutes(new Date(), -setting);
  } else if (filterSource === "DASHBOARD") {
    const setting =
      dashboardDateRangeAggregationSettings[
        option as keyof typeof dashboardDateRangeAggregationSettings
      ];

    return addMinutes(new Date(), -setting.minutes);
  }
  return undefined;
};

export function isValidDashboardDateRangeAggregationOption(
  value?: string,
): value is DashboardDateRangeAggregationOption {
  if (!value) return false;
  return (DASHBOARD_AGGREGATION_OPTIONS as readonly string[]).includes(value);
}

export function isValidTableDateRangeAggregationOption(
  value?: string,
): value is TableDateRangeAggregationOption {
  if (!value) return false;
  return (TABLE_AGGREGATION_OPTIONS as readonly string[]).includes(value);
}

export const getAvailableDashboardOptions = (
  logRetentionDays: number,
): DashboardDateRangeAggregationOption[] => {
  return DASHBOARD_AGGREGATION_OPTIONS.filter((option) => {
    const setting = dashboardDateRangeAggregationSettings[option];
    return setting.minutes <= logRetentionDays * 24 * 60;
  });
};

export const getMinimumPlanForOption = (
  option: DashboardDateRangeAggregationOption,
): {
  planName: string;
  planDisplayName: string;
  retentionDays: number;
} | null => {
  const setting = dashboardDateRangeAggregationSettings[option];
  const requiredDays = Math.ceil(setting.minutes / (24 * 60));

  if (requiredDays <= 3) {
    return null;
  } else if (requiredDays <= 7) {
    return {
      planName: "starter",
      planDisplayName: "Starter",
      retentionDays: 7,
    };
  } else if (requiredDays <= 30) {
    return { planName: "team", planDisplayName: "Team", retentionDays: 30 };
  } else {
    return {
      planName: "enterprise",
      planDisplayName: "Enterprise",
      retentionDays: 999,
    };
  }
};

export const getOptionDescription = (
  option: DashboardDateRangeAggregationOption,
  userRetentionDays?: number,
): string => {
  const minPlan = getMinimumPlanForOption(option);
  const setting = dashboardDateRangeAggregationSettings[option];
  const requiredDays = Math.ceil(setting.minutes / (24 * 60));

  if (!minPlan) {
    return `Available in your current plan`;
  }

  if (userRetentionDays && requiredDays <= userRetentionDays) {
    return `Available in your current plan`;
  } else {
    const retentionText =
      minPlan.planName === "enterprise"
        ? "custom log retention"
        : `${minPlan.retentionDays} days retention`;
    return `Requires ${minPlan.planDisplayName} plan (${retentionText})`;
  }
};

export const findClosestDashboardInterval = (
  dateRange: DateRange,
): DashboardDateRangeAggregationOption | undefined => {
  if (!dateRange.from || !dateRange.to) return undefined;
  const duration = dateRange.to.getTime() - dateRange.from.getTime();

  const diffs = DASHBOARD_AGGREGATION_OPTIONS.map((interval) => {
    const { minutes } = dashboardDateRangeAggregationSettings[interval];
    return {
      interval,
      diff: Math.abs(duration - minutes * 60 * 1000),
    };
  });

  diffs.sort((a, b) => a.diff - b.diff);

  return diffs[0]?.interval;
};
