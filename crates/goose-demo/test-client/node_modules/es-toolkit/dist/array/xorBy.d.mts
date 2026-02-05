/**
 * Computes the symmetric difference between two arrays using a custom mapping function.
 * The symmetric difference is the set of elements which are in either of the arrays,
 * but not in their intersection, determined by the result of the mapping function.
 *
 * @template T - Type of elements in the input arrays.
 * @template U - Type of the values returned by the mapping function.
 *
 * @param {T[]} arr1 - The first array.
 * @param {T[]} arr2 - The second array.
 * @param {(item: T) => U} mapper - The function to map array elements to comparison values.
 * @returns {T[]} An array containing the elements that are present in either `arr1` or `arr2` but not in both, based on the values returned by the mapping function.
 *
 * @example
 * // Custom mapping function for objects with an 'id' property
 * const idMapper = obj => obj.id;
 * xorBy([{ id: 1 }, { id: 2 }], [{ id: 2 }, { id: 3 }], idMapper);
 * // Returns [{ id: 1 }, { id: 3 }]
 */
declare function xorBy<T, U>(arr1: readonly T[], arr2: readonly T[], mapper: (item: T) => U): T[];

export { xorBy };
