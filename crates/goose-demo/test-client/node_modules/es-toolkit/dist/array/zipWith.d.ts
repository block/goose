/**
 * Combines multiple arrays into a single array using a custom combiner function.
 *
 * This function takes multiple arrays and a combiner function, and returns a new array where each element
 * is the result of applying the combiner function to the corresponding elements of the input arrays.
 *
 * @template T - The type of elements in the first array.
 * @template R - The type of elements in the resulting array.
 * @param {T[]} arr1 - The first array to zip.
 * @param {(item: T, index: number) => R} combine - The combiner function that takes corresponding elements from each array, their index, and returns a single value.
 * @returns {R[]} A new array where each element is the result of applying the combiner function to the corresponding elements of the input arrays.
 *
 * @example
 * // Example usage with two arrays:
 * const arr1 = [1, 2, 3];
 * const arr2 = [4, 5, 6];
 * const result = zipWith(arr1, arr2, (a, b) => a + b);
 * // result will be [5, 7, 9]
 *
 * @example
 * // Example usage with three arrays:
 * const arr1 = [1, 2];
 * const arr2 = [3, 4];
 * const arr3 = [5, 6];
 * const result = zipWith(arr1, arr2, arr3, (a, b, c) => `${a}${b}${c}`);
 * // result will be [`135`, `246`]
 */
declare function zipWith<T, R>(arr1: readonly T[], combine: (item: T, index: number) => R): R[];
/**
 * Combines two arrays into a single array using a custom combiner function.
 *
 * @template T - The type of elements in the first array.
 * @template U - The type of elements in the second array.
 * @template R - The type of elements in the resulting array.
 * @param {T[]} arr1 - The first array to zip.
 * @param {U[]} arr2 - The second array to zip.
 * @param {(item1: T, item2: U, index: number) => R} combine - The combiner function that takes corresponding elements from each array, their index, and returns a single value.
 * @returns {R[]} A new array where each element is the result of applying the combiner function to the corresponding elements of the input arrays.
 */
declare function zipWith<T, U, R>(arr1: readonly T[], arr2: readonly U[], combine: (item1: T, item2: U, index: number) => R): R[];
/**
 * Combines three arrays into a single array using a custom combiner function.
 *
 * @template T - The type of elements in the first array.
 * @template U - The type of elements in the second array.
 * @template V - The type of elements in the third array.
 * @template R - The type of elements in the resulting array.
 * @param {T[]} arr1 - The first array to zip.
 * @param {U[]} arr2 - The second array to zip.
 * @param {V[]} arr3 - The third array to zip.
 * @param {(item1: T, item2: U, item3: V, index: number) => R} combine - The combiner function that takes corresponding elements from each array, their index, and returns a single value.
 * @returns {R[]} A new array where each element is the result of applying the combiner function to the corresponding elements of the input arrays.
 */
declare function zipWith<T, U, V, R>(arr1: readonly T[], arr2: readonly U[], arr3: readonly V[], combine: (item1: T, item2: U, item3: V, index: number) => R): R[];
/**
 * Combines four arrays into a single array using a custom combiner function.
 *
 * @template T - The type of elements in the first array.
 * @template U - The type of elements in the second array.
 * @template V - The type of elements in the third array.
 * @template W - The type of elements in the fourth array.
 * @template R - The type of elements in the resulting array.
 * @param {T[]} arr1 - The first array to zip.
 * @param {U[]} arr2 - The second array to zip.
 * @param {V[]} arr3 - The third array to zip.
 * @param {W[]} arr4 - The fourth array to zip.
 * @param {(item1: T, item2: U, item3: V, item4: W, index: number) => R} combine - The combiner function that takes corresponding elements from each array, their index, and returns a single value.
 * @returns {R[]} A new array where each element is the result of applying the combiner function to the corresponding elements of the input arrays.
 */
declare function zipWith<T, U, V, W, R>(arr1: readonly T[], arr2: readonly U[], arr3: readonly V[], arr4: readonly W[], combine: (item1: T, item2: U, item3: V, item4: W, index: number) => R): R[];

export { zipWith };
