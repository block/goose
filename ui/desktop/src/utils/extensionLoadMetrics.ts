import { ExtensionLoadResult } from '../api/types.gen';

type ExtensionLoadMetric = {
  averageMs: number;
  count: number;
  lastMs: number;
  updatedAt: number;
};

const STORAGE_KEY = 'goose-extension-load-metrics-v1';

const canUseLocalStorage = () => {
  try {
    return typeof window !== 'undefined' && !!window.localStorage;
  } catch {
    return false;
  }
};

const readMetrics = (): Record<string, ExtensionLoadMetric> => {
  if (!canUseLocalStorage()) return {};

  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw);
    if (!parsed || typeof parsed !== 'object') return {};
    return parsed as Record<string, ExtensionLoadMetric>;
  } catch {
    return {};
  }
};

const writeMetrics = (metrics: Record<string, ExtensionLoadMetric>) => {
  if (!canUseLocalStorage()) return;

  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(metrics));
  } catch {
    // localStorage write may fail in restricted environments
  }
};

export const updateExtensionLoadMetrics = (results: ExtensionLoadResult[]) => {
  const metrics = readMetrics();
  const now = Date.now();

  for (const result of results) {
    if (!result.success || result.durationMs === undefined || result.durationMs === null) {
      continue;
    }

    const duration = Math.max(0, result.durationMs);
    const previous = metrics[result.name];
    const count = previous ? previous.count : 0;
    const averageMs = previous
      ? (previous.averageMs * count + duration) / (count + 1)
      : duration;

    metrics[result.name] = {
      averageMs,
      count: count + 1,
      lastMs: duration,
      updatedAt: now,
    };
  }

  writeMetrics(metrics);
  return metrics;
};

export const estimateExtensionLoadTimes = (extensionNames: string[]) => {
  const metrics = readMetrics();
  const perExtensionMs: Record<string, number | undefined> = {};
  let totalMs = 0;
  let hasEstimate = false;

  for (const name of extensionNames) {
    const metric = metrics[name];
    const estimate = metric
      ? metric.count > 1
        ? metric.averageMs
        : metric.lastMs
      : undefined;

    if (estimate !== undefined) {
      totalMs += estimate;
      hasEstimate = true;
    }

    perExtensionMs[name] = estimate;
  }

  return {
    totalMs: hasEstimate ? Math.round(totalMs) : undefined,
    perExtensionMs,
  };
};
