export { formatTokenCount } from '../../utils/usageFormatting';

export function formatTokenCountPrecise(count: number): string {
  if (count < 1000) return String(count);
  const format = (value: number, suffix: string) => {
    const rounded = Math.round(value * 10) / 10;
    const text = Number.isInteger(rounded) ? String(rounded) : rounded.toFixed(1);
    return `${text}${suffix}`;
  };
  if (count < 999_950) return format(count / 1000, 'k');
  return format(count / 1_000_000, 'M');
}

export function formatPercentOf(part: number, total: number): string {
  if (total <= 0) return '0%';
  const percent = (part / total) * 100;
  const rounded = Math.round(percent);
  if (part > 0 && rounded === 0) return '<1%';
  return `${rounded}%`;
}
