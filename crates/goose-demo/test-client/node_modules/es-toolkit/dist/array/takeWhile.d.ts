/**
 * Returns a new array containing the leading elements of the provided array
 * that satisfy the provided predicate function. It stops taking elements as soon
 * as an element does not satisfy the predicate.
 *
 * @template T - The type of elements in the array.
 * @param {T[]} arr - The array to process.
 * @param {(element: T, index: number, array: readonly T[]) => boolean} shouldContinueTaking - The predicate function that is called with each element, its index, and the array. Elements are included in the result as long as this function returns true.
 * @returns {T[]} A new array containing the leading elements that satisfy the predicate.
 *
 * @example
 * // Returns [1, 2]
 * takeWhile([1, 2, 3, 4], x => x < 3);
 *
 * @example
 * // Returns []
 * takeWhile([1, 2, 3, 4], x => x > 3);
 *
 * @example
 * // Using index parameter
 * takeWhile([10, 20, 30, 40], (x, index) => index < 2);
 * // Returns: [10, 20]
 */
declare function takeWhile<T>(arr: readonly T[], shouldContinueTaking: (element: T, index: number, array: readonly T[]) => boolean): T[];

export { takeWhile };
