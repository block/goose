interface DebounceOptions {
    /**
     * An optional AbortSignal to cancel the debounced function.
     */
    signal?: AbortSignal;
    /**
     * An optional array specifying whether the function should be invoked on the leading edge, trailing edge, or both.
     * If `edges` includes "leading", the function will be invoked at the start of the delay period.
     * If `edges` includes "trailing", the function will be invoked at the end of the delay period.
     * If both "leading" and "trailing" are included, the function will be invoked at both the start and end of the delay period.
     * @default ["trailing"]
     */
    edges?: Array<'leading' | 'trailing'>;
}
interface DebouncedFunction<F extends (...args: any[]) => void> {
    (...args: Parameters<F>): void;
    /**
     * Schedules the execution of the debounced function after the specified debounce delay.
     * This method resets any existing timer, ensuring that the function is only invoked
     * after the delay has elapsed since the last call to the debounced function.
     * It is typically called internally whenever the debounced function is invoked.
     *
     * @returns {void}
     */
    schedule: () => void;
    /**
     * Cancels any pending execution of the debounced function.
     * This method clears the active timer and resets any stored context or arguments.
     */
    cancel: () => void;
    /**
     * Immediately invokes the debounced function if there is a pending execution.
     * This method executes the function right away if there is a pending execution.
     */
    flush: () => void;
}
/**
 * Creates a debounced function that delays invoking the provided function until after `debounceMs` milliseconds
 * have elapsed since the last time the debounced function was invoked. The debounced function also has a `cancel`
 * method to cancel any pending execution.
 *
 * @template F - The type of function.
 * @param {F} func - The function to debounce.
 * @param {number} debounceMs - The number of milliseconds to delay.
 * @param {DebounceOptions} options - The options object
 * @param {AbortSignal} options.signal - An optional AbortSignal to cancel the debounced function.
 * @returns A new debounced function with a `cancel` method.
 *
 * @example
 * const debouncedFunction = debounce(() => {
 *   console.log('Function executed');
 * }, 1000);
 *
 * // Will log 'Function executed' after 1 second if not called again in that time
 * debouncedFunction();
 *
 * // Will not log anything as the previous call is canceled
 * debouncedFunction.cancel();
 *
 * // With AbortSignal
 * const controller = new AbortController();
 * const signal = controller.signal;
 * const debouncedWithSignal = debounce(() => {
 *  console.log('Function executed');
 * }, 1000, { signal });
 *
 * debouncedWithSignal();
 *
 * // Will cancel the debounced function call
 * controller.abort();
 */
declare function debounce<F extends (...args: any[]) => void>(func: F, debounceMs: number, { signal, edges }?: DebounceOptions): DebouncedFunction<F>;

export { type DebounceOptions, type DebouncedFunction, debounce };
