/**
 * Defers invoking the `func` until the current call stack has cleared. Any additional arguments are provided to func when it's invoked.
 *
 * @param {(...args: any[]) => any} func The function to defer.
 * @param {...any[]} args The arguments to invoke `func` with.
 * @returns {number} Returns the timer id.
 *
 * @example
 * defer(console.log, 'deferred');
 * // => Logs 'deferred' after the current call stack has cleared.
 */
declare function defer(func: (...args: any[]) => any, ...args: any[]): number;

export { defer };
