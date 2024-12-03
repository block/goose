import throttleFunction from 'throttleit';

export function throttle<T extends (...args: any[]) => any>(
  fn: T,
  waitMs: number | undefined,
): T {
  return waitMs != null ? throttleFunction(fn, waitMs) : fn;
}
