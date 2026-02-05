/**
 * Creates a function that limits the number of times the given function (`func`) can be called.
 *
 * @template F - The type of the function to be invoked.
 * @param {number} n - The number of times the returned function is allowed to call `func` before stopping.
 * - If `n` is 0, `func` will never be called.
 * - If `n` is a positive integer, `func` will be called up to `n-1` times.
 * @param {F} func - The function to be called with the limit applied.
 * @returns {(...args: Parameters<F>) => ReturnType<F> | undefined} - A new function that:
 * - Tracks the number of calls.
 * - Invokes `func` until the `n-1`-th call.
 * - Returns `undefined` if the number of calls reaches or exceeds `n`, stopping further calls.
 * @throws {Error} - Throw an error if `n` is negative.
 * @example
 *
 * const beforeFn = before(3, () => {
 *  console.log("called");
 * })
 *
 * // Will log 'called'.
 * beforeFn();
 *
 * // Will log 'called'.
 * beforeFn();
 *
 * // Will not log anything.
 * beforeFn();
 */
declare function before<F extends (...args: any[]) => any>(n: number, func: F): (...args: Parameters<F>) => ReturnType<F> | undefined;

export { before };
