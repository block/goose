/**
 * Flattens array up to depth times.
 *
 * @template T
 * @param {ArrayLike<T> | null | undefined} value - The array to flatten.
 * @param {number} depth - The maximum recursion depth.
 * @returns {any[]} Returns the new flattened array.
 *
 * @example
 * flatten([1, [2, [3, [4]], 5]], 2);
 * // => [1, 2, 3, [4], 5]
 */
declare function flatten<T>(value: ArrayLike<T | readonly T[]> | null | undefined): T[];

export { flatten };
