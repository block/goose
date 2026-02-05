/**
 * Returns the intersection of two arrays based on a custom equality function.
 *
 * This function takes two arrays and a custom equality function. It returns a new array containing
 * the elements from the first array that have matching elements in the second array, as determined
 * by the custom equality function. It effectively filters out any elements from the first array that
 * do not have corresponding matches in the second array according to the equality function.
 *
 * @template T - The type of elements in the first array.
 * @template U - The type of elements in the second array.
 * @param {T[]} firstArr - The first array to compare.
 * @param {U[]} secondArr - The second array to compare.
 * @param {(x: T, y: U) => boolean} areItemsEqual - A custom function to determine if two elements are equal.
 * This function takes two arguments, one from each array, and returns `true` if the elements are considered equal, and `false` otherwise.
 * @returns {T[]} A new array containing the elements from the first array that have corresponding matches in the second array according to the custom equality function.
 *
 * @example
 * const array1 = [{ id: 1 }, { id: 2 }, { id: 3 }];
 * const array2 = [{ id: 2 }, { id: 4 }];
 * const areItemsEqual = (a, b) => a.id === b.id;
 * const result = intersectionWith(array1, array2, areItemsEqual);
 * // result will be [{ id: 2 }] since this element has a matching id in both arrays.
 *
 * @example
 * const array1 = [
 *   { id: 1, name: 'jane' },
 *   { id: 2, name: 'amy' },
 *   { id: 3, name: 'michael' },
 * ];
 * const array2 = [2, 4];
 * const areItemsEqual = (a, b) => a.id === b;
 * const result = intersectionWith(array1, array2, areItemsEqual);
 * // result will be [{ id: 2, name: 'amy' }] since this element has a matching id that is equal to seconds array's element.
 */
declare function intersectionWith<T, U>(firstArr: readonly T[], secondArr: readonly U[], areItemsEqual: (x: T, y: U) => boolean): T[];

export { intersectionWith };
