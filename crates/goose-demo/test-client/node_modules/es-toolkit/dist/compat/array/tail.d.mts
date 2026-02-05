/**
 * Gets all but the first element of array.
 *
 * @template T
 * @param {readonly [unknown, ...T]} array - The array to query.
 * @returns {T} Returns the slice of array.
 *
 * @example
 * tail([1, 2, 3]);
 * // => [2, 3]
 */
declare function tail<T extends unknown[]>(array: readonly [unknown, ...T]): T;
/**
 * Gets all but the first element of array.
 *
 * @template T
 * @param {ArrayLike<T> | null | undefined} array - The array to query.
 * @returns {T[]} Returns the slice of array.
 *
 * @example
 * tail([1, 2, 3]);
 * // => [2, 3]
 */
declare function tail<T>(array: ArrayLike<T> | null | undefined): T[];

export { tail };
