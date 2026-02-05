/**
 * Combines one array into a single array using a custom combiner function.
 *
 * @template T - The type of elements in the first array.
 * @template R - The type of elements in the resulting array.
 * @param {ArrayLike<T>} arr1 - The first array to zip.
 * @param {(item: T) => R} combine - The combiner function that takes corresponding elements from each array and returns a single value.
 * @returns {R[]} A new array where each element is the result of applying the combiner function to the corresponding elements of the input arrays.
 */
declare function zipWith<T, R>(arr1: ArrayLike<T>, combine: (item: T) => R): R[];
/**
 * Combines two arrays into a single array using a custom combiner function.
 *
 * @template T - The type of elements in the first array.
 * @template U - The type of elements in the second array.
 * @template R - The type of elements in the resulting array.
 * @param {ArrayLike<T>} arr1 - The first array to zip.
 * @param {ArrayLike<U>} arr2 - The second array to zip.
 * @param {(item1: T, item2: U) => R} combine - The combiner function that takes corresponding elements from each array and returns a single value.
 * @returns {R[]} A new array where each element is the result of applying the combiner function to the corresponding elements of the input arrays.
 */
declare function zipWith<T, U, R>(arr1: ArrayLike<T>, arr2: ArrayLike<U>, combine: (item1: T, item2: U) => R): R[];
/**
 * Combines three arrays into a single array using a custom combiner function.
 *
 * @template T - The type of elements in the first array.
 * @template U - The type of elements in the second array.
 * @template V - The type of elements in the third array.
 * @template R - The type of elements in the resulting array.
 * @param {ArrayLike<T>} arr1 - The first array to zip.
 * @param {ArrayLike<U>} arr2 - The second array to zip.
 * @param {ArrayLike<V>} arr3 - The third array to zip.
 * @param {(item1: T, item2: U, item3: V) => R} combine - The combiner function that takes corresponding elements from each array and returns a single value.
 * @returns {R[]} A new array where each element is the result of applying the combiner function to the corresponding elements of the input arrays.
 */
declare function zipWith<T, U, V, R>(arr1: ArrayLike<T>, arr2: ArrayLike<U>, arr3: ArrayLike<V>, combine: (item1: T, item2: U, item3: V) => R): R[];
/**
 * Combines four arrays into a single array using a custom combiner function.
 *
 * @template T - The type of elements in the first array.
 * @template U - The type of elements in the second array.
 * @template V - The type of elements in the third array.
 * @template W - The type of elements in the fourth array.
 * @template R - The type of elements in the resulting array.
 * @param {ArrayLike<T>} arr1 - The first array to zip.
 * @param {ArrayLike<U>} arr2 - The second array to zip.
 * @param {ArrayLike<V>} arr3 - The third array to zip.
 * @param {ArrayLike<W>} arr4 - The fourth array to zip.
 * @param {(item1: T, item2: U, item3: V, item4: W) => R} combine - The combiner function that takes corresponding elements from each array and returns a single value.
 * @returns {R[]} A new array where each element is the result of applying the combiner function to the corresponding elements of the input arrays.
 */
declare function zipWith<T, U, V, W, R>(arr1: ArrayLike<T>, arr2: ArrayLike<U>, arr3: ArrayLike<V>, arr4: ArrayLike<W>, combine: (item1: T, item2: U, item3: V, item4: W) => R): R[];
/**
 * Combines five arrays into a single array using a custom combiner function.
 *
 * @template T - The type of elements in the first array.
 * @template U - The type of elements in the second array.
 * @template V - The type of elements in the third array.
 * @template W - The type of elements in the fourth array.
 * @template X - The type of elements in the fifth array.
 * @template R - The type of elements in the resulting array.
 * @param {ArrayLike<T>} arr1 - The first array to zip.
 * @param {ArrayLike<U>} arr2 - The second array to zip.
 * @param {ArrayLike<V>} arr3 - The third array to zip.
 * @param {ArrayLike<W>} arr4 - The fourth array to zip.
 * @param {ArrayLike<X>} arr5 - The fifth array to zip.
 * @param {(item1: T, item2: U, item3: V, item4: W, item5: X) => R} combine - The combiner function that takes corresponding elements from each array and returns a single value.
 * @returns {R[]} A new array where each element is the result of applying the combiner function to the corresponding elements of the input arrays.
 */
declare function zipWith<T, U, V, W, X, R>(arr1: ArrayLike<T>, arr2: ArrayLike<U>, arr3: ArrayLike<V>, arr4: ArrayLike<W>, arr5: ArrayLike<X>, combine: (item1: T, item2: U, item3: V, item4: W, item5: X) => R): R[];
/**
 * Combines multiple arrays into a single array using a custom combiner function.
 *
 * This function takes one array and a variable number of additional arrays,
 * applying the provided combiner function to the corresponding elements of each array.
 * If the input arrays are of different lengths, the resulting array will have the length
 * of the longest input array, with undefined values for missing elements.
 *
 * @template T - The type of elements in the input arrays.
 * @template R - The type of elements in the resulting array.
 * @param {Array<((...group: T[]) => R) | ArrayLike<T> | null | undefined>} combine - The combiner function that takes corresponding elements from each array and returns a single value.
 * @returns {R[]} A new array where each element is the result of applying the combiner function to the corresponding elements of the input arrays.
 *
 * @example
 * const arr1 = [1, 2, 3];
 * const arr2 = ['a', 'b', 'c'];
 * const result = zipWith(arr1, arr2, (num, char) => `${num}${char}`);
 * // result will be ['1a', '2b', '3c']
 */
declare function zipWith<T, R>(...combine: Array<((...group: T[]) => R) | ArrayLike<T> | null | undefined>): R[];

export { zipWith };
