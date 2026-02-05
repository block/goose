/**
 * Returns a new array containing the first `count` elements from the input array `arr`.
 * If `count` is greater than the length of `arr`, the entire array is returned.
 *
 * @template T - Type of elements in the input array.
 *
 * @param {T[]} arr - The array to take elements from.
 * @param {number} count - The number of elements to take.
 * @param {unknown} guard - If truthy, ignores `count` and defaults to 1.
 * @returns {T[]} A new array containing the first `count` elements from `arr`.
 *
 * @example
 * // Returns [1, 2, 3]
 * take([1, 2, 3, 4, 5], 3);
 *
 * @example
 * // Returns ['a', 'b']
 * take(['a', 'b', 'c'], 2);
 *
 * @example
 * // Returns [1, 2, 3]
 * take([1, 2, 3], 5);
 *
 * @example
 * // Returns [[1], [1], [1]]
 * const arr = [1, 2, 3];
 * const result = arr.map((v, i, array) => take(array, i, true));
 */
declare function take<T>(arr: readonly T[], count?: number, guard?: unknown): T[];

export { take };
