/**
 * Combines multiple arrays into a single array of tuples.
 *
 * This function takes multiple arrays and returns a new array where each element is a tuple
 * containing the corresponding elements from the input arrays. If the input arrays are of
 * different lengths, the resulting array will have the length of the longest input array,
 * with undefined values for missing elements.
 *
 * @template T
 * @param {T[]} arr1 - The first array to zip.
 * @returns {Array<[T]>} A new array of tuples containing the corresponding elements from the input arrays.
 *
 * @example
 * const arr1 = [1, 2, 3];
 * const result = zip(arr1);
 * // result will be [[1], [2], [3]]
 */
declare function zip<T>(arr1: readonly T[]): Array<[T]>;
/**
 * Combines multiple arrays into a single array of tuples.
 *
 * This function takes multiple arrays and returns a new array where each element is a tuple
 * containing the corresponding elements from the input arrays. If the input arrays are of
 * different lengths, the resulting array will have the length of the longest input array,
 * with undefined values for missing elements.
 *
 * @template T, U
 * @param {T[]} arr1 - The first array to zip.
 * @param {U[]} arr2 - The second array to zip.
 * @returns {Array<[T, U]>} A new array of tuples containing the corresponding elements from the input arrays.
 *
 * @example
 * const arr1 = [1, 2, 3];
 * const arr2 = ['a', 'b', 'c'];
 * const result = zip(arr1, arr2);
 * // result will be [[1, 'a'], [2, 'b'], [3, 'c']]
 */
declare function zip<T, U>(arr1: readonly T[], arr2: readonly U[]): Array<[T, U]>;
/**
 * Combines multiple arrays into a single array of tuples.
 *
 * This function takes multiple arrays and returns a new array where each element is a tuple
 * containing the corresponding elements from the input arrays. If the input arrays are of
 * different lengths, the resulting array will have the length of the longest input array,
 * with undefined values for missing elements.
 *
 * @template T, U, V
 * @param {T[]} arr1 - The first array to zip.
 * @param {U[]} arr2 - The second array to zip.
 * @param {V[]} arr3 - The third array to zip.
 * @returns {Array<[T, U, V]>} A new array of tuples containing the corresponding elements from the input arrays.
 *
 * @example
 * const arr1 = [1, 2, 3];
 * const arr2 = ['a', 'b', 'c'];
 * const arr3 = [true, false];
 * const result = zip(arr1, arr2, arr3);
 * // result will be [[1, 'a', true], [2, 'b', false], [3, 'c', undefined]]
 */
declare function zip<T, U, V>(arr1: readonly T[], arr2: readonly U[], arr3: readonly V[]): Array<[T, U, V]>;
/**
 * Combines multiple arrays into a single array of tuples.
 *
 * This function takes multiple arrays and returns a new array where each element is a tuple
 * containing the corresponding elements from the input arrays. If the input arrays are of
 * different lengths, the resulting array will have the length of the longest input array,
 * with undefined values for missing elements.
 *
 * @template T, U, V, W
 * @param {T[]} arr1 - The first array to zip.
 * @param {U[]} arr2 - The second array to zip.
 * @param {V[]} arr3 - The third array to zip.
 * @param {W[]} arr4 - The fourth array to zip.
 * @returns {Array<[T, U, V, W]>} A new array of tuples containing the corresponding elements from the input arrays.
 *
 * @example
 * const arr1 = [1, 2, 3];
 * const arr2 = ['a', 'b', 'c'];
 * const arr3 = [true, false];
 * const arr4 = [null, null, null];
 * const result = zip(arr1, arr2, arr3, arr4);
 * // result will be [[1, 'a', true, null], [2, 'b', false, null], [3, 'c', undefined, null]]
 */
declare function zip<T, U, V, W>(arr1: readonly T[], arr2: readonly U[], arr3: readonly V[], arr4: readonly W[]): Array<[T, U, V, W]>;
/**
 * Combines multiple arrays into a single array of tuples.
 *
 * This function takes multiple arrays and returns a new array where each element is a tuple
 * containing the corresponding elements from the input arrays. If the input arrays are of
 * different lengths, the resulting array will have the length of the longest input array,
 * with undefined values for missing elements.
 *
 * @template T
 * @param {...Array<readonly T[]>} arrs - The arrays to zip together.
 * @returns {T[][]} A new array of tuples containing the corresponding elements from the input arrays.
 *
 * @example
 * const arr1 = [1, 2, 3];
 * const arr2 = ['a', 'b', 'c'];
 * const arr3 = [true, false];
 * const result = zip(arr1, arr2, arr3);
 * // result will be [[1, 'a', true], [2, 'b', false], [3, 'c', undefined]]
 */
declare function zip<T>(...arrs: Array<readonly T[]>): T[][];

export { zip };
