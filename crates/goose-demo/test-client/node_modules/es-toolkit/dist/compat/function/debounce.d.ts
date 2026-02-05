interface DebounceSettings {
    /**
     * If `true`, the function will be invoked on the leading edge of the timeout.
     * @default false
     */
    leading?: boolean | undefined;
    /**
     * The maximum time `func` is allowed to be delayed before it's invoked.
     * @default Infinity
     */
    maxWait?: number | undefined;
    /**
     * If `true`, the function will be invoked on the trailing edge of the timeout.
     * @default true
     */
    trailing?: boolean | undefined;
}
interface DebounceSettingsLeading extends DebounceSettings {
    leading: true;
}
interface DebouncedFunc<T extends (...args: any[]) => any> {
    /**
     * Call the original function, but applying the debounce rules.
     *
     * If the debounced function can be run immediately, this calls it and returns its return
     * value.
     *
     * Otherwise, it returns the return value of the last invocation, or undefined if the debounced
     * function was not invoked yet.
     */
    (...args: Parameters<T>): ReturnType<T> | undefined;
    /**
     * Throw away any pending invocation of the debounced function.
     */
    cancel(): void;
    /**
     * If there is a pending invocation of the debounced function, invoke it immediately and return
     * its return value.
     *
     * Otherwise, return the value from the last invocation, or undefined if the debounced function
     * was never invoked.
     */
    flush(): ReturnType<T> | undefined;
}
interface DebouncedFuncLeading<T extends (...args: any[]) => any> extends DebouncedFunc<T> {
    (...args: Parameters<T>): ReturnType<T>;
    flush(): ReturnType<T>;
}
/**
 * Creates a debounced function that delays invoking the provided function until after `debounceMs` milliseconds
 * have elapsed since the last time the debounced function was invoked. The debounced function also has a `cancel`
 * method to cancel any pending execution.
 *
 * You can set the debounced function to run at the start (`leading`) or end (`trailing`) of the delay period.
 * If `leading` is true, the function runs immediately on the first call.
 * If `trailing` is true, the function runs after `debounceMs` milliseconds have passed since the last call.
 * If both `leading` and `trailing` are true, the function runs at both the start and end, but it must be called at least twice within `debounceMs` milliseconds for this to happen
 * (since one debounced function call cannot trigger the function twice).
 *
 * You can also set a `maxWait` time, which is the maximum time the function is allowed to be delayed before it is called.
 *
 * @template F - The type of function.
 * @param {F} func - The function to debounce.
 * @param {number} debounceMs - The number of milliseconds to delay.
 * @param {DebounceOptions} options - The options object
 * @param {AbortSignal} options.signal - An optional AbortSignal to cancel the debounced function.
 * @param {boolean} options.leading - If `true`, the function will be invoked on the leading edge of the timeout.
 * @param {boolean} options.trailing - If `true`, the function will be invoked on the trailing edge of the timeout.
 * @param {number} options.maxWait - The maximum time `func` is allowed to be delayed before it's invoked.
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
declare function debounce<T extends (...args: any) => any>(func: T, wait: number | undefined, options: DebounceSettingsLeading): DebouncedFuncLeading<T>;
/**
 * Creates a debounced function that delays invoking the provided function until after `debounceMs` milliseconds
 * have elapsed since the last time the debounced function was invoked. The debounced function also has a `cancel`
 * method to cancel any pending execution.
 *
 * You can set the debounced function to run at the start (`leading`) or end (`trailing`) of the delay period.
 * If `leading` is true, the function runs immediately on the first call.
 * If `trailing` is true, the function runs after `debounceMs` milliseconds have passed since the last call.
 * If both `leading` and `trailing` are true, the function runs at both the start and end, but it must be called at least twice within `debounceMs` milliseconds for this to happen
 * (since one debounced function call cannot trigger the function twice).
 *
 * You can also set a `maxWait` time, which is the maximum time the function is allowed to be delayed before it is called.
 *
 * @template F - The type of function.
 * @param {F} func - The function to debounce.
 * @param {number} debounceMs - The number of milliseconds to delay.
 * @param {DebounceOptions} options - The options object
 * @param {AbortSignal} options.signal - An optional AbortSignal to cancel the debounced function.
 * @param {boolean} options.leading - If `true`, the function will be invoked on the leading edge of the timeout.
 * @param {boolean} options.trailing - If `true`, the function will be invoked on the trailing edge of the timeout.
 * @param {number} options.maxWait - The maximum time `func` is allowed to be delayed before it's invoked.
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
declare function debounce<T extends (...args: any) => any>(func: T, wait?: number, options?: DebounceSettings): DebouncedFunc<T>;

export { type DebouncedFunc, type DebouncedFuncLeading, debounce };
