/**
 * Computes the difference between two arrays based on a custom equality function.
 *
 * This function takes two arrays and a custom comparison function. It returns a new array containing
 * the elements that are present in the first array but not in the second array. The comparison to determine
 * if elements are equal is made using the provided custom function.
 *
 * @template T, U
 * @param {T[]} firstArr - The array from which to get the difference.
 * @param {U[]} secondArr - The array containing elements to exclude from the first array.
 * @param {(x: T, y: U) => boolean} areItemsEqual - A function to determine if two items are equal.
 * @returns {T[]} A new array containing the elements from the first array that do not match any elements in the second array
 * according to the custom equality function.
 *
 * @example
 * const array1 = [{ id: 1 }, { id: 2 }, { id: 3 }];
 * const array2 = [{ id: 2 }, { id: 4 }];
 * const areItemsEqual = (a, b) => a.id === b.id;
 * const result = differenceWith(array1, array2, areItemsEqual);
 * // result will be [{ id: 1 }, { id: 3 }] since the elements with id 2 are considered equal and are excluded from the result.
 *
 * @example
 * const array1 = [{ id: 1 }, { id: 2 }, { id: 3 }];
 * const array2 = [2, 4];
 * const areItemsEqual = (a, b) => a.id === b;
 * const result = differenceWith(array1, array2, areItemsEqual);
 * // result will be [{ id: 1 }, { id: 3 }] since the element with id 2 is considered equal to the second array's element and is excluded from the result.
 */
declare function differenceWith<T, U>(firstArr: readonly T[], secondArr: readonly U[], areItemsEqual: (x: T, y: U) => boolean): T[];

export { differenceWith };
