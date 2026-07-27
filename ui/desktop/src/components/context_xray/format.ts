export { formatTokenCount } from '../../utils/usageFormatting';

export function formatPercentOf(part: number, total: number): string {
  if (total <= 0) return '0%';
  const percent = (part / total) * 100;
  const rounded = Math.round(percent);
  if (part > 0 && rounded === 0) return '<1%';
  return `${rounded}%`;
}
