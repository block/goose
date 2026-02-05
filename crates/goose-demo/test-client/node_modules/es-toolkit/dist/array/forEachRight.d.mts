/**
 * Iterates over elements of 'arr' from right to left and invokes 'callback' for each element.
 *
 * @template T - The type of elements in the array.
 * @param {T[]} arr - The array to iterate over.
 * @param {(value: T, index: number, arr: T[]) => void} callback - The function invoked per iteration.
 * The callback function receives three arguments:
 *  - 'value': The current element being processed in the array.
 *  - 'index': The index of the current element being processed in the array.
 *  - 'arr': The array 'forEachRight' was called upon.
 *
 * @example
 * const array = [1, 2, 3];
 * const result: number[] = [];
 *
 * // Use the forEachRight function to iterate through the array and add each element to the result array.
 * forEachRight(array, (value) => {
 *  result.push(value);
 * })
 *
 * console.log(result) // Output: [3, 2, 1]
 */
declare function forEachRight<T>(arr: T[], callback: (value: T, index: number, arr: T[]) => void): void;
/**
 * Iterates over elements of 'arr' from right to left and invokes 'callback' for each element.
 *
 * @template T - The type of elements in the array.
 * @param {T[]} arr - The array to iterate over.
 * @param {(value: T, index: number, arr: T[]) => void} callback - The function invoked per iteration.
 * The callback function receives three arguments:
 *  - 'value': The current element being processed in the array.
 *  - 'index': The index of the current element being processed in the array.
 *  - 'arr': The array 'forEachRight' was called upon.
 *
 * @example
 * const array = [1, 2, 3];
 * const result: number[] = [];
 *
 * // Use the forEachRight function to iterate through the array and add each element to the result array.
 * forEachRight(array, (value) => {
 *  result.push(value);
 * })
 *
 * console.log(result) // Output: [3, 2, 1]
 */
declare function forEachRight<T>(arr: readonly T[], callback: (value: T, index: number, arr: readonly T[]) => void): void;

export { forEachRight };
