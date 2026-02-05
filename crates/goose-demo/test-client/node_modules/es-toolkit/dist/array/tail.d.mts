/**
 * Returns an empty array when the input is a single-element array.
 *
 * @template T - The type of the single element in the array.
 * @param {[T]} arr - The single-element array to process.
 * @returns {[]} An empty array.
 *
 * @example
 * const arr = [1];
 * const result = tail(arr);
 * // result will be []
 */
declare function tail<T>(arr: readonly [T]): [];
/**
 * Returns an empty array when the input is an empty array.
 *
 * @template T - The type of elements in the array.
 * @param {[]} arr - The empty array to process.
 * @returns {[]} An empty array.
 *
 * @example
 * const arr = [];
 * const result = tail(arr);
 * // result will be []
 */
declare function tail(arr: readonly []): [];
/**
 * Returns a new array with all elements except for the first when the input is a tuple array.
 *
 * @template T - The type of the first element in the tuple array.
 * @template U - The type of the remaining elements in the tuple array.
 * @param {[T, ...U[]]} arr - The tuple array to process.
 * @returns {U[]} A new array containing all elements of the input array except for the first one.
 *
 * @example
 * const arr = [1, 2, 3];
 * const result = tail(arr);
 * // result will be [2, 3]
 */
declare function tail<T, U>(arr: readonly [T, ...U[]]): U[];
/**
 * Returns a new array with all elements except for the first.
 *
 * This function takes an array and returns a new array containing all the elements
 * except for the first one. If the input array is empty or has only one element,
 * an empty array is returned.
 *
 * @template T - The type of elements in the array.
 * @param {T[]} arr - The array to get the tail of.
 * @returns {T[]} A new array containing all elements of the input array except for the first one.
 *
 * @example
 * const arr1 = [1, 2, 3];
 * const result = tail(arr1);
 * // result will be [2, 3]
 *
 * const arr2 = [1];
 * const result2 = tail(arr2);
 * // result2 will be []
 *
 * const arr3 = [];
 * const result3 = tail(arr3);
 * // result3 will be []
 */
declare function tail<T>(arr: readonly T[]): T[];

export { tail };
