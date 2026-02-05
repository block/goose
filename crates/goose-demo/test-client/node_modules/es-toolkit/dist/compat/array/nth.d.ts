/**
 * Gets the element at index `n` of `array`. If `n` is negative, the nth element from the end is returned.
 *
 * @param {ArrayLike<T> | null | undefined} array - The array to query.
 * @param {number} [n=0] - The index of the element to return.
 * @return {T | undefined} Returns the nth element of `array`.
 *
 * @example
 * nth([1, 2, 3], 1); // => 2
 * nth([1, 2, 3], -1); // => 3
 */
declare function nth<T>(array: ArrayLike<T> | null | undefined, n?: number): T | undefined;

export { nth };
