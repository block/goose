/**
 * Creates an array that excludes all specified values.
 *
 * It correctly excludes `NaN`, as it compares values using [SameValueZero](https://tc39.es/ecma262/multipage/abstract-operations.html#sec-samevaluezero).
 *
 * @template T The type of elements in the array.
 * @param {ArrayLike<T> | null | undefined} array - The array to filter.
 * @param {...T[]} values - The values to exclude.
 * @returns {T[]} A new array without the specified values.
 *
 * @example
 * // Removes the specified values from the array
 * without([1, 2, 3, 4, 5], 2, 4);
 * // Returns: [1, 3, 5]
 *
 * @example
 * // Removes specified string values from the array
 * without(['a', 'b', 'c', 'a'], 'a');
 * // Returns: ['b', 'c']
 */
declare function without<T>(array: ArrayLike<T> | null | undefined, ...values: T[]): T[];

export { without };
