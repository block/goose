/**
 * Returns a new array containing all elements except the last one from the input array.
 * If the input array is empty or has only one element, the function returns an empty array.
 *
 * @template T The type of elements in the array.
 * @param {ArrayLike<T> | null | undefined} arr - The input array.
 * @returns {T[]} A new array containing all but the last element of the input array.
 *
 * @example
 * const arr = [1, 2, 3, 4];
 * const result = initial(arr);
 * // result will be [1, 2, 3]
 */
declare function initial<T>(arr: ArrayLike<T> | null | undefined): T[];

export { initial };
