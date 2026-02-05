/**
 * Removes a specified number of elements from the beginning of an array and returns the rest.
 *
 * This function takes an array and a number, and returns a new array with the specified number
 * of elements removed from the start.
 *
 * @template T - The type of elements in the array.
 * @param {ArrayLike<T> | null | undefined} collection - The array from which to drop elements.
 * @param {number} itemsCount - The number of elements to drop from the beginning of the array.
 * @param {unknown} [guard] - Enables use as an iteratee for methods like `_.map`.
 * @returns {T[]} A new array with the specified number of elements removed from the start.
 *
 * @example
 * const array = [1, 2, 3, 4, 5];
 * const result = drop(array, 2);
 * result will be [3, 4, 5] since the first two elements are dropped.
 */
declare function drop<T>(array: ArrayLike<T> | null | undefined, n?: number): T[];

export { drop };
