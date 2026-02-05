/**
 * Combines multiple arrays into a single array of tuples.
 *
 * This function takes multiple arrays and returns a new array where each element is a tuple
 * containing the corresponding elements from the input arrays. If the input arrays are of
 * different lengths, the resulting array will have the length of the longest input array,
 * with undefined values for missing elements.
 *
 * @template T, U
 * @param {ArrayLike<T>} arr1 - The first array to zip.
 * @param {ArrayLike<U>} arr2 - The second array to zip.
 * @returns {Array<[T | undefined, U | undefined]>} A new array of tuples containing the corresponding elements from the input arrays.
 *
 * @example
 * const arr1 = [1, 2, 3];
 * const arr2 = ['a', 'b', 'c'];
 * const result = zip(arr1, arr2);
 * // result will be [[1, 'a'], [2, 'b'], [3, 'c']]
 */
/**
 * Creates an array of grouped elements, the first of which contains the first elements of the given arrays,
 * the second of which contains the second elements of the given arrays, and so on.
 *
 * @template T, U
 * @param {ArrayLike<T>} arr1 - The first array to zip.
 * @param {ArrayLike<U>} arr2 - The second array to zip.
 * @returns {Array<[T | undefined, U | undefined]>} Returns the new array of grouped elements.
 *
 * @example
 * zip([1, 2], ['a', 'b']);
 * // => [[1, 'a'], [2, 'b']]
 */
declare function zip<T, U>(arr1: ArrayLike<T>, arr2: ArrayLike<U>): Array<[T | undefined, U | undefined]>;
/**
 * Combines multiple arrays into a single array of tuples.
 *
 * This function takes multiple arrays and returns a new array where each element is a tuple
 * containing the corresponding elements from the input arrays. If the input arrays are of
 * different lengths, the resulting array will have the length of the longest input array,
 * with undefined values for missing elements.
 *
 * @template T, U, V
 * @param {ArrayLike<T>} arr1 - The first array to zip.
 * @param {ArrayLike<U>} arr2 - The second array to zip.
 * @param {ArrayLike<V>} arr3 - The third array to zip.
 * @returns {Array<[T | undefined, U | undefined, V | undefined]>} A new array of tuples containing the corresponding elements from the input arrays.
 *
 * @example
 * const arr1 = [1, 2, 3];
 * const arr2 = ['a', 'b', 'c'];
 * const arr3 = [true, false];
 * const result = zip(arr1, arr2, arr3);
 * // result will be [[1, 'a', true], [2, 'b', false], [3, 'c', undefined]]
 */
/**
 * Creates an array of grouped elements, the first of which contains the first elements of the given arrays,
 * the second of which contains the second elements of the given arrays, and so on.
 *
 * @template T, U, V
 * @param {ArrayLike<T>} arr1 - The first array to zip.
 * @param {ArrayLike<U>} arr2 - The second array to zip.
 * @param {ArrayLike<V>} arr3 - The third array to zip.
 * @returns {Array<[T | undefined, U | undefined, V | undefined]>} Returns the new array of grouped elements.
 *
 * @example
 * zip([1, 2], ['a', 'b'], [true, false]);
 * // => [[1, 'a', true], [2, 'b', false]]
 */
declare function zip<T, U, V>(arr1: ArrayLike<T>, arr2: ArrayLike<U>, arr3: ArrayLike<V>): Array<[T | undefined, U | undefined, V | undefined]>;
/**
 * Combines multiple arrays into a single array of tuples.
 *
 * This function takes multiple arrays and returns a new array where each element is a tuple
 * containing the corresponding elements from the input arrays. If the input arrays are of
 * different lengths, the resulting array will have the length of the longest input array,
 * with undefined values for missing elements.
 *
 * @template T, U, V, W
 * @param {ArrayLike<T>} arr1 - The first array to zip.
 * @param {ArrayLike<U>} arr2 - The second array to zip.
 * @param {ArrayLike<V>} arr3 - The third array to zip.
 * @param {ArrayLike<W>} arr4 - The fourth array to zip.
 * @returns {Array<[T | undefined, U | undefined, V | undefined, W | undefined]>} A new array of tuples containing the corresponding elements from the input arrays.
 *
 * @example
 * const arr1 = [1, 2, 3];
 * const arr2 = ['a', 'b', 'c'];
 * const arr3 = [true, false];
 * const arr4 = [null, null, null];
 * const result = zip(arr1, arr2, arr3, arr4);
 * // result will be [[1, 'a', true, null], [2, 'b', false, null], [3, 'c', undefined, null]]
 */
/**
 * Creates an array of grouped elements, the first of which contains the first elements of the given arrays,
 * the second of which contains the second elements of the given arrays, and so on.
 *
 * @template T, U, V, W
 * @param {ArrayLike<T>} arr1 - The first array to zip.
 * @param {ArrayLike<U>} arr2 - The second array to zip.
 * @param {ArrayLike<V>} arr3 - The third array to zip.
 * @param {ArrayLike<W>} arr4 - The fourth array to zip.
 * @returns {Array<[T | undefined, U | undefined, V | undefined, W | undefined]>} Returns the new array of grouped elements.
 *
 * @example
 * zip([1], ['a'], [true], [null]);
 * // => [[1, 'a', true, null]]
 */
declare function zip<T, U, V, W>(arr1: ArrayLike<T>, arr2: ArrayLike<U>, arr3: ArrayLike<V>, arr4: ArrayLike<W>): Array<[T | undefined, U | undefined, V | undefined, W | undefined]>;
/**
 * Combines multiple arrays into a single array of tuples.
 *
 * This function takes multiple arrays and returns a new array where each element is a tuple
 * containing the corresponding elements from the input arrays. If the input arrays are of
 * different lengths, the resulting array will have the length of the longest input array,
 * with undefined values for missing elements.
 *
 * @template T, U, V, W
 * @param {ArrayLike<T>} arr1 - The first array to zip.
 * @param {ArrayLike<U>} arr2 - The second array to zip.
 * @param {ArrayLike<V>} arr3 - The third array to zip.
 * @param {ArrayLike<W>} arr4 - The fourth array to zip.
 * @param {ArrayLike<X>} arr5 - The fifth array to zip.
 * @returns {Array<[T | undefined, U | undefined, V | undefined, W | undefined, X | undefined]>} A new array of tuples containing the corresponding elements from the input arrays.
 *
 * @example
 * const arr1 = [1, 2, 3];
 * const arr2 = ['a', 'b', 'c'];
 * const arr3 = [true, false];
 * const arr4 = [null, null, null];
 * const arr5 = [undefined, undefined, undefined];
 * const result = zip(arr1, arr2, arr3, arr4, arr5);
 * // result will be [[1, 'a', true, null, undefined], [2, 'b', false, null, undefined], [3, 'c', undefined, null, undefined]]
 */
/**
 * Creates an array of grouped elements, the first of which contains the first elements of the given arrays,
 * the second of which contains the second elements of the given arrays, and so on.
 *
 * @template T, U, V, W, X
 * @param {ArrayLike<T>} arr1 - The first array to zip.
 * @param {ArrayLike<U>} arr2 - The second array to zip.
 * @param {ArrayLike<V>} arr3 - The third array to zip.
 * @param {ArrayLike<W>} arr4 - The fourth array to zip.
 * @param {ArrayLike<X>} arr5 - The fifth array to zip.
 * @returns {Array<[T | undefined, U | undefined, V | undefined, W | undefined, X | undefined]>} Returns the new array of grouped elements.
 *
 * @example
 * zip([1], ['a'], [true], [null], [undefined]);
 * // => [[1, 'a', true, null, undefined]]
 */
declare function zip<T, U, V, W, X>(arr1: ArrayLike<T>, arr2: ArrayLike<U>, arr3: ArrayLike<V>, arr4: ArrayLike<W>, arr5: ArrayLike<X>): Array<[T | undefined, U | undefined, V | undefined, W | undefined, X | undefined]>;
/**
 * Combines multiple arrays into a single array of tuples.
 *
 * This function takes multiple arrays and returns a new array where each element is a tuple
 * containing the corresponding elements from the input arrays. If the input arrays are of
 * different lengths, the resulting array will have the length of the longest input array,
 * with undefined values for missing elements.
 *
 * @template T
 * @param {Array<ArrayLike<any> | null | undefined>} arrays - The arrays to zip.
 * @returns {Array<Array<T | undefined>>} A new array of tuples containing the corresponding elements from the input arrays.
 *
 * @example
 * const arr1 = [1, 2, 3];
 * const arr2 = ['a', 'b', 'c'];
 * const arr3 = [true, false];
 * const arr4 = [null, null, null];
 * const arr5 = [undefined, undefined, undefined];
 * const result = zip(arr1, arr2, arr3, arr4, arr5);
 * // result will be [[1, 'a', true, null, undefined], [2, 'b', false, null, undefined], [3, 'c', undefined, null, undefined]]
 */
/**
 * Creates an array of grouped elements, the first of which contains the first elements of the given arrays,
 * the second of which contains the second elements of the given arrays, and so on.
 *
 * @template T
 * @param {...Array<ArrayLike<T> | null | undefined>} arrays - The arrays to process.
 * @returns {Array<Array<T | undefined>>} Returns the new array of grouped elements.
 *
 * @example
 * zip([1, 2], ['a', 'b'], [true, false]);
 * // => [[1, 'a', true], [2, 'b', false]]
 */
declare function zip<T>(...arrays: Array<ArrayLike<T> | null | undefined>): Array<Array<T | undefined>>;

export { zip };
