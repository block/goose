/**
 * Creates a function that negates the result of the predicate function.
 *
 * @template T - The type of the arguments array.
 * @param {(...args: T) => boolean} predicate - The predicate to negate.
 * @returns {(...args: T) => boolean} The new negated function.
 *
 * @example
 * function isEven(n) {
 *   return n % 2 == 0;
 * }
 *
 * filter([1, 2, 3, 4, 5, 6], negate(isEven));
 * // => [1, 3, 5]
 */
declare function negate<T extends any[]>(predicate: (...args: T) => boolean): (...args: T) => boolean;

export { negate };
