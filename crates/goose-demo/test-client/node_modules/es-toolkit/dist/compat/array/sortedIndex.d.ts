/**
 * Uses a binary search to determine the lowest index at which `value`
 * should be inserted into `array` in order to maintain its sort order.
 *
 * @category Array
 * @param {ArrayLike<T> | null | undefined} array The sorted array to inspect.
 * @param {T} value The value to evaluate.
 * @returns {number} Returns the index at which `value` should be inserted
 *  into `array`.
 * @example
 * sortedIndex([30, 50], 40)
 * // => 1
 */
declare function sortedIndex<T>(array: ArrayLike<T> | null | undefined, value: T): number;
/**
 * Uses a binary search to determine the lowest index at which `value`
 * should be inserted into `array` in order to maintain its sort order.
 *
 * @category Array
 * @param {ArrayLike<T> | null | undefined} array The sorted array to inspect.
 * @param {T} value The value to evaluate.
 * @returns {number} Returns the index at which `value` should be inserted
 *  into `array`.
 * @example
 * sortedIndex([30, 50], 40)
 * // => 1
 */
declare function sortedIndex<T>(array: ArrayLike<T> | null | undefined, value: T): number;

export { sortedIndex };
