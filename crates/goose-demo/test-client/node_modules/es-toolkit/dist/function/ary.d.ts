/**
 * Creates a function that invokes func, with up to n arguments, ignoring any additional arguments.
 *
 * @template F - The type of the function.
 * @param {F} func - The function to cap arguments for.
 * @param {number} n - The arity cap.
 * @returns {(...args: any[]) => ReturnType<F>} Returns the new capped function.
 *
 * @example
 * function fn(a: number, b: number, c: number) {
 *   return Array.from(arguments);
 * }
 *
 * ary(fn, 0)(1, 2, 3) // []
 * ary(fn, 1)(1, 2, 3) // [1]
 * ary(fn, 2)(1, 2, 3) // [1, 2]
 * ary(fn, 3)(1, 2, 3) // [1, 2, 3]
 */
declare function ary<F extends (...args: any[]) => any>(func: F, n: number): (...args: any[]) => ReturnType<F>;

export { ary };
