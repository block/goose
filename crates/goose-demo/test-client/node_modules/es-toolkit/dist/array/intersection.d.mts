/**
 * Returns the intersection of two arrays.
 *
 * This function takes two arrays and returns a new array containing the elements that are
 * present in both arrays. It effectively filters out any elements from the first array that
 * are not found in the second array.
 *
 * @template T - The type of elements in the array.
 * @param {T[]} firstArr - The first array to compare.
 * @param {T[]} secondArr - The second array to compare.
 * @returns {T[]} A new array containing the elements that are present in both arrays.
 *
 * @example
 * const array1 = [1, 2, 3, 4, 5];
 * const array2 = [3, 4, 5, 6, 7];
 * const result = intersection(array1, array2);
 * // result will be [3, 4, 5] since these elements are in both arrays.
 */
declare function intersection<T>(firstArr: readonly T[], secondArr: readonly T[]): T[];

export { intersection };
