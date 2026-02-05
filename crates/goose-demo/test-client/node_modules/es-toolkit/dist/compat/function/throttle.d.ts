import { DebouncedFuncLeading, DebouncedFunc } from './debounce.js';

interface ThrottleSettings {
    /**
     * If `true`, the function will be invoked on the leading edge of the timeout.
     * @default true
     */
    leading?: boolean | undefined;
    /**
     * If `true`, the function will be invoked on the trailing edge of the timeout.
     * @default true
     */
    trailing?: boolean | undefined;
}
type ThrottleSettingsLeading = (ThrottleSettings & {
    leading: true;
}) | Omit<ThrottleSettings, 'leading'>;
/**
 * Creates a throttled function that only invokes the provided function at most once
 * per every `throttleMs` milliseconds. Subsequent calls to the throttled function
 * within the wait time will not trigger the execution of the original function.
 *
 * @template F - The type of function.
 * @param {F} func - The function to throttle.
 * @param {number} throttleMs - The number of milliseconds to throttle executions to.
 * @param {ThrottleOptions} options - The options object
 * @param {AbortSignal} options.signal - An optional AbortSignal to cancel the throttled function.
 * @param {boolean} options.leading - If `true`, the function will be invoked on the leading edge of the timeout.
 * @param {boolean} options.trailing - If `true`, the function will be invoked on the trailing edge of the timeout.
 * @returns {(...args: Parameters<F>) => void} A new throttled function that accepts the same parameters as the original function.
 *
 * @example
 * const throttledFunction = throttle(() => {
 *   console.log('Function executed');
 * }, 1000);
 *
 * // Will log 'Function executed' immediately
 * throttledFunction();
 *
 * // Will not log anything as it is within the throttle time
 * throttledFunction();
 *
 * // After 1 second
 * setTimeout(() => {
 *   throttledFunction(); // Will log 'Function executed'
 * }, 1000);
 */
declare function throttle<T extends (...args: any) => any>(func: T, throttleMs?: number, options?: ThrottleSettingsLeading): DebouncedFuncLeading<T>;
/**
 * Creates a throttled function that only invokes the provided function at most once
 * per every `throttleMs` milliseconds. Subsequent calls to the throttled function
 * within the wait time will not trigger the execution of the original function.
 *
 * @template F - The type of function.
 * @param {F} func - The function to throttle.
 * @param {number} throttleMs - The number of milliseconds to throttle executions to.
 * @param {ThrottleOptions} options - The options object
 * @param {AbortSignal} options.signal - An optional AbortSignal to cancel the throttled function.
 * @param {boolean} options.leading - If `true`, the function will be invoked on the leading edge of the timeout.
 * @param {boolean} options.trailing - If `true`, the function will be invoked on the trailing edge of the timeout.
 * @returns {(...args: Parameters<F>) => void} A new throttled function that accepts the same parameters as the original function.
 *
 * @example
 * const throttledFunction = throttle(() => {
 *   console.log('Function executed');
 * }, 1000);
 *
 * // Will log 'Function executed' immediately
 * throttledFunction();
 *
 * // Will not log anything as it is within the throttle time
 * throttledFunction();
 *
 * // After 1 second
 * setTimeout(() => {
 *   throttledFunction(); // Will log 'Function executed'
 * }, 1000);
 */
declare function throttle<T extends (...args: any) => any>(func: T, throttleMs?: number, options?: ThrottleSettings): DebouncedFunc<T>;

export { throttle };
