/**
 * Creates an array of unique values from two given arrays based on a custom equality function.
 *
 * This function takes two arrays and a custom equality function, merges the arrays, and returns
 * a new array containing only the unique values as determined by the custom equality function.
 *
 * @template T - The type of elements in the array.
 * @param {T[]} arr1 - The first array to merge and filter for unique values.
 * @param {T[]} arr2 - The second array to merge and filter for unique values.
 * @param {(item1: T, item2: T) => boolean} areItemsEqual - A custom function to determine if two elements are equal.
 * It takes two arguments and returns `true` if the elements are considered equal, and `false` otherwise.
 * @returns {T[]} A new array of unique values based on the custom equality function.
 *
 * @example
 * const array1 = [{ id: 1 }, { id: 2 }];
 * const array2 = [{ id: 2 }, { id: 3 }];
 * const areItemsEqual = (a, b) => a.id === b.id;
 * const result = unionWith(array1, array2, areItemsEqual);
 * // result will be [{ id: 1 }, { id: 2 }, { id: 3 }] since { id: 2 } is considered equal in both arrays
 */
declare function unionWith<T>(arr1: readonly T[], arr2: readonly T[], areItemsEqual: (item1: T, item2: T) => boolean): T[];

export { unionWith };
