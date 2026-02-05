/**
 * Creates a function that invokes `func`, with the `this` binding and arguments
 * of the created function, while it's called less than `n` times. Subsequent
 * calls to the created function return the result of the last `func` invocation.
 *
 * @template F - The type of the function to be invoked.
 * @param {number} n - The number of times the returned function is allowed to call `func` before stopping.
 * - If `n` is 0, `func` will never be called.
 * - If `n` is a positive integer, `func` will be called up to `n-1` times.
 * @param {F} func - The function to be called with the limit applied.
 * @returns {(...args: Parameters<F>) => ReturnType<F> } - A new function that:
 * - Tracks the number of calls.
 * - Invokes `func` until the `n-1`-th call.
 * - Returns last result of `func`, if `n` is reached.
 * @throws {TypeError} - If `func` is not a function.
 * @example
 * let count = 0;
 * const before3 = before(3, () => ++count);
 *
 * before3(); // => 1
 * before3(); // => 2
 * before3(); // => 2
 */
declare function before<F extends (...args: any[]) => any>(n: number, func: F): F;

export { before };
