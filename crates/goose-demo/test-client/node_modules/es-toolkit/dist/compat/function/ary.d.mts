/**
 * Creates a function that invokes func, with up to `n` arguments, ignoring any additional arguments.
 * If `n` is not provided, it defaults to the function's length.
 *
 * @param {Function} func - The function to cap arguments for.
 * @param {number} [n] - The arity cap. Defaults to func.length.
 * @returns {Function} Returns the new capped function.
 *
 * @example
 * function fn(a: number, b: number, c: number) {
 *   return Array.from(arguments);
 * }
 *
 * // Cap at 2 arguments
 * const capped = ary(fn, 2);
 * capped(1, 2, 3); // [1, 2]
 *
 * // Default to function length
 * const defaultCap = ary(fn);
 * defaultCap(1, 2, 3); // [1, 2, 3]
 */
declare function ary(func: (...args: any[]) => any, n?: number): (...args: any[]) => any;

export { ary };
