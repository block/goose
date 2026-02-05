/**
 * Returns the last element of an array.
 *
 * This function takes an array and returns the last element of the array.
 * If the array is empty, the function returns `undefined`.
 *
 * Unlike some implementations, this function is optimized for performance
 * by directly accessing the last index of the array.
 *
 * @template T - The type of elements in the array.
 * @param {ArrayLike<T> | null | undefined} arr - The array from which to get the last element.
 * @returns {T | undefined} The last element of the array, or `undefined` if the array is empty.
 *
 * @example
 * const arr = [1, 2, 3];
 * const lastElement = last(arr);
 * // lastElement will be 3
 *
 * const emptyArr: number[] = [];
 * const noElement = last(emptyArr);
 * // noElement will be undefined
 */
declare function last<T>(array: ArrayLike<T> | null | undefined): T | undefined;

export { last };
