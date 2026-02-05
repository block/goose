/**
 * Creates a function that invokes given functions and returns their results as an array.
 *
 * @param {Array<Iteratee | Iteratee[]>} iteratees - The iteratees to invoke.
 * @returns {(...args: any[]) => unknown[]} Returns the new function.
 *
 * @example
 * const func = over([Math.max, Math.min]);
 * const func2 = over(Math.max, Math.min); // same as above
 * func(1, 2, 3, 4);
 * // => [4, 1]
 * func2(1, 2, 3, 4);
 * // => [4, 1]
 *
 * const func = over(['a', 'b']);
 * func({ a: 1, b: 2 });
 * // => [1, 2]
 *
 * const func = over([{ a: 1 }, { b: 2 }]);
 * func({ a: 1, b: 2 });
 * // => [true, false]
 *
 * const func = over([['a', 1], ['b', 2]]);
 * func({ a: 1, b: 2 });
 * // => [true, true]
 */
declare function over<T>(...iteratees: Array<((...args: any[]) => T) | ReadonlyArray<(...args: any[]) => T>>): (...args: any[]) => T[];

export { over };
