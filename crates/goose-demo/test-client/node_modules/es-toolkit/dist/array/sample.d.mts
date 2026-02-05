/**
 * Returns a random element from an array.
 *
 * This function takes an array and returns a single element selected randomly from the array.
 *
 * @template T - The type of elements in the array.
 * @param {T[]} arr - The array to sample from.
 * @returns {T} A random element from the array.
 *
 * @example
 * const array = [1, 2, 3, 4, 5];
 * const randomElement = sample(array);
 * // randomElement will be one of the elements from the array, selected randomly.
 */
declare function sample<T>(arr: readonly T[]): T;

export { sample };
