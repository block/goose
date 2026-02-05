/**
 * Checks if the `subset` array is entirely contained within the `superset` array.
 *
 *
 * @template T - The type of elements contained in the arrays.
 * @param {T[]} superset - The array that may contain all elements of the subset.
 * @param {T[]} subset - The array to check against the superset.
 * @returns {boolean} - Returns `true` if all elements of the `subset` are present in the `superset`, otherwise returns `false`.
 *
 * @example
 * ```typescript
 * const superset = [1, 2, 3, 4, 5];
 * const subset = [2, 3, 4];
 * isSubset(superset, subset); // true
 * ```
 *
 * @example
 * ```typescript
 * const superset = ['a', 'b', 'c'];
 * const subset = ['a', 'd'];
 * isSubset(superset, subset); // false
 * ```
 */
declare function isSubset<T>(superset: readonly T[], subset: readonly T[]): boolean;

export { isSubset };
