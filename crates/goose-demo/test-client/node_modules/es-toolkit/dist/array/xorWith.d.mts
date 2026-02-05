/**
 * Computes the symmetric difference between two arrays using a custom equality function.
 * The symmetric difference is the set of elements which are in either of the arrays,
 * but not in their intersection.
 *
 * @template T - Type of elements in the input arrays.
 *
 * @param {T[]} arr1 - The first array.
 * @param {T[]} arr2 - The second array.
 * @param {(item1: T, item2: T) => boolean} areElementsEqual - The custom equality function to compare elements.
 * @returns {T[]} An array containing the elements that are present in either `arr1` or `arr2` but not in both, based on the custom equality function.
 *
 * @example
 * // Custom equality function for objects with an 'id' property
 * const areObjectsEqual = (a, b) => a.id === b.id;
 * xorWith([{ id: 1 }, { id: 2 }], [{ id: 2 }, { id: 3 }], areObjectsEqual);
 * // Returns [{ id: 1 }, { id: 3 }]
 */
declare function xorWith<T>(arr1: readonly T[], arr2: readonly T[], areElementsEqual: (item1: T, item2: T) => boolean): T[];

export { xorWith };
