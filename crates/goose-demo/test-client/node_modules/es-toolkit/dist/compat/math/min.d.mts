/**
 * Finds the element in an array that has the minimum value.
 *
 * @template T - The type of elements in the array.
 * @param {ArrayLike<T> | null | undefined} [items] - The array of elements to search. Defaults to an empty array.
 * @returns {T | undefined} - The element with the minimum value, or undefined if the array is empty.
 */
declare function min<T>(items: ArrayLike<T> | null | undefined): T | undefined;

export { min };
