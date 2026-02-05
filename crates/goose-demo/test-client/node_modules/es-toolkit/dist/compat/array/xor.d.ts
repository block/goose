/**
 * Computes the symmetric difference of the provided arrays, returning an array of elements
 * that exist in only one of the arrays.
 *
 * @template T - The type of elements in the arrays.
 * @param {...(ArrayLike<T> | null | undefined)} arrays - The arrays to compare.
 * @returns {T[]} An array containing the elements that are present in only one of the provided `arrays`.
 *
 * @example
 * // Returns [1, 2, 5, 6]
 * xor([1, 2, 3, 4], [3, 4, 5, 6]);
 *
 * @example
 * // Returns ['a', 'c']
 * xor(['a', 'b'], ['b', 'c']);
 *
 * @example
 * // Returns [1, 3, 5]
 * xor([1, 2], [2, 3], [4, 5]);
 */
declare function xor<T>(...arrays: Array<ArrayLike<T> | null | undefined>): T[];

export { xor };
