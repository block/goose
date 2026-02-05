/**
 * Returns a new array containing only the unique elements from the original array,
 * based on the values returned by the mapper function.
 *
 * When duplicates are found, the first occurrence is kept and the rest are discarded.
 *
 * @template T - The type of elements in the array.
 * @template U - The type of mapped elements.
 * @param {T[]} arr - The array to process.
 * @param {(item: T) => U} mapper - The function used to convert the array elements.
 * @returns {T[]} A new array containing only the unique elements from the original array, based on the values returned by the mapper function.
 *
 * @example
 * ```ts
 * uniqBy([1.2, 1.5, 2.1, 3.2, 5.7, 5.3, 7.19], Math.floor);
 * // [1.2, 2.1, 3.2, 5.7, 7.19]
 * ```
 *
 * @example
 * const array = [
 *   { category: 'fruit', name: 'apple' },
 *   { category: 'fruit', name: 'banana' },
 *   { category: 'vegetable', name: 'carrot' },
 * ];
 * uniqBy(array, item => item.category).length
 * // 2
 * ```
 */
declare function uniqBy<T, U>(arr: readonly T[], mapper: (item: T, index: number, array: readonly T[]) => U): T[];

export { uniqBy };
