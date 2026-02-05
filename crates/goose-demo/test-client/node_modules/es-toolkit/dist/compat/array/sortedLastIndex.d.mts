/**
 * Uses a binary search to determine the highest index at which `value`
 * should be inserted into `array` in order to maintain its sort order.
 *
 * @category Array
 * @param {ArrayLike<T> | null | undefined} array The sorted array to inspect.
 * @param {T} value The value to evaluate.
 * @returns {number} Returns the index at which `value` should be inserted
 *  into `array`.
 * @example
 * sortedIndex([4, 5, 5, 5, 6], 5)
 * // => 4
 */
declare function sortedLastIndex<T>(array: ArrayLike<T> | null | undefined, value: T): number;

export { sortedLastIndex };
