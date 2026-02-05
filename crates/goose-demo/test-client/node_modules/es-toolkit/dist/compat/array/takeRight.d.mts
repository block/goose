/**
 * Creates a slice of array with n elements taken from the end.
 *
 * @template T
 * @param {ArrayLike<T> | null | undefined} array - The array to query.
 * @param {number} [n=1] - The number of elements to take.
 * @returns {T[]} Returns the slice of array.
 *
 * @example
 * takeRight([1, 2, 3]);
 * // => [3]
 *
 * @example
 * takeRight([1, 2, 3], 2);
 * // => [2, 3]
 *
 * @example
 * takeRight([1, 2, 3], 5);
 * // => [1, 2, 3]
 *
 * @example
 * takeRight([1, 2, 3], 0);
 * // => []
 */
declare function takeRight<T>(array: ArrayLike<T> | null | undefined, n?: number): T[];

export { takeRight };
