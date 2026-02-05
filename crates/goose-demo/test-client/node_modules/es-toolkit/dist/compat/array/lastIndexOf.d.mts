/**
 * Gets the index at which the last occurrence of value is found in array.
 *
 * @template T
 * @param {ArrayLike<T> | null | undefined} array - The array to inspect.
 * @param {T} value - The value to search for.
 * @param {true | number} [fromIndex] - The index to search from or true to search from the end.
 * @returns {number} Returns the index of the matched value, else -1.
 *
 * @example
 * lastIndexOf([1, 2, 1, 2], 2);
 * // => 3
 *
 * lastIndexOf([1, 2, 1, 2], 2, 2);
 * // => 1
 *
 * lastIndexOf([1, 2, 1, 2], 2, true);
 * // => 3
 */
declare function lastIndexOf<T>(array: ArrayLike<T> | null | undefined, searchElement: T, fromIndex?: true | number): number;

export { lastIndexOf };
