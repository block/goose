/**
 * Computes the difference between an array and multiple arrays.
 *
 * @template T
 * @param {ArrayLike<T> | undefined | null} arr - The primary array from which to derive the difference. This is the main array
 * from which elements will be compared and filtered.
 * @param {Array<ArrayLike<T>>} values - Multiple arrays containing elements to be excluded from the primary array.
 * These arrays will be flattened into a single array, and each element in this array will be checked against the primary array.
 * If a match is found, that element will be excluded from the result.
 * @returns {T[]} A new array containing the elements that are present in the primary array but not
 * in the flattened array.
 *
 * @example
 * const array1 = [1, 2, 3, 4, 5];
 * const array2 = [2, 4];
 * const array3 = [5, 6];
 * const result = difference(array1, array2, array3);
 * // result will be [1, 3] since 2, 4, and 5 are in the other arrays and are excluded from the result.
 *
 * @example
 * const arrayLike1 = { 0: 1, 1: 2, 2: 3, length: 3 };
 * const arrayLike2 = { 0: 2, 1: 4, length: 2 };
 * const result = difference(arrayLike1, arrayLike2);
 * // result will be [1, 3] since 2 is in both array-like objects and is excluded from the result.
 */
declare function difference<T>(arr: ArrayLike<T> | undefined | null, ...values: Array<ArrayLike<T>>): T[];

export { difference };
