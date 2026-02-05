/**
 * Returns the intersection of two arrays based on a mapping function.
 *
 * This function takes two arrays and a mapping function. It returns a new array containing
 * the elements from the first array that, when mapped using the provided function, have matching
 * mapped elements in the second array. It effectively filters out any elements from the first array
 * that do not have corresponding mapped values in the second array.
 *
 * @template T - The type of elements in the first array.
 * @template U - The type of elements in the second array.
 * @param {T[]} firstArr - The first array to compare.
 * @param {U[]} secondArr - The second array to compare.
 * @param {(item: T | U) => unknown} mapper - A function to map the elements of both arrays for comparison.
 * @returns {T[]} A new array containing the elements from the first array that have corresponding mapped values in the second array.
 *
 * @example
 * const array1 = [{ id: 1 }, { id: 2 }, { id: 3 }];
 * const array2 = [{ id: 2 }, { id: 4 }];
 * const mapper = item => item.id;
 * const result = intersectionBy(array1, array2, mapper);
 * // result will be [{ id: 2 }] since only this element has a matching id in both arrays.
 *
 * @example
 * const array1 = [
 *   { id: 1, name: 'jane' },
 *   { id: 2, name: 'amy' },
 *   { id: 3, name: 'michael' },
 * ];
 * const array2 = [2, 4];
 * const mapper = item => (typeof item === 'object' ? item.id : item);
 * const result = intersectionBy(array1, array2, mapper);
 * // result will be [{ id: 2, name: 'amy' }] since only this element has a matching id that is equal to seconds array's element.
 */
declare function intersectionBy<T, U>(firstArr: readonly T[], secondArr: readonly U[], mapper: (item: T | U) => unknown): T[];

export { intersectionBy };
