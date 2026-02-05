/**
 * Checks if the `subset` array is entirely contained within the `superset` array based on a custom equality function.
 *
 * This function takes two arrays and a custom comparison function. It returns a boolean indicating
 * whether all elements in the subset array are present in the superset array, as determined by the provided
 * custom equality function.
 *
 * @template T - The type of elements contained in the arrays.
 * @param {T[]} superset - The array that may contain all elements of the subset.
 * @param {T[]} subset - The array to check against the superset.
 * @param {(x: T, y: T) => boolean} areItemsEqual - A function to determine if two items are equal.
 * @returns {boolean} - Returns `true` if all elements of the subset are present in the superset
 * according to the custom equality function, otherwise returns `false`.
 *
 * @example
 * ```typescript
 * const superset = [{ id: 1 }, { id: 2 }, { id: 3 }];
 * const subset = [{ id: 2 }, { id: 1 }];
 * const areItemsEqual = (a, b) => a.id === b.id;
 * isSubsetWith(superset, subset, areItemsEqual); // true
 * ```
 *
 * @example
 * ```typescript
 * const superset = [{ id: 1 }, { id: 2 }, { id: 3 }];
 * const subset = [{ id: 4 }];
 * const areItemsEqual = (a, b) => a.id === b.id;
 * isSubsetWith(superset, subset, areItemsEqual); // false
 * ```
 */
declare function isSubsetWith<T>(superset: readonly T[], subset: readonly T[], areItemsEqual: (x: T, y: T) => boolean): boolean;

export { isSubsetWith };
