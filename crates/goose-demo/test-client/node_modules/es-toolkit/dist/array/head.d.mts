/**
 * Returns the first element of an array.
 *
 * This function takes an array and returns the first element of the array.
 * If the array is empty, the function returns `undefined`.
 *
 * @template T - The type of elements in the array.
 * @param {[T, ...T[]]} arr - A non-empty array from which to get the first element.
 * @returns {T} The first element of the array.
 *
 * @example
 * const arr = [1, 2, 3];
 * const firstElement = head(arr);
 * // firstElement will be 1
 */
declare function head<T>(arr: readonly [T, ...T[]]): T;
/**
 * Returns the first element of an array or `undefined` if the array is empty.
 *
 * This function takes an array and returns the first element of the array.
 * If the array is empty, the function returns `undefined`.
 *
 * @template T - The type of elements in the array.
 * @param {T[]} arr - The array from which to get the first element.
 * @returns {T | undefined} The first element of the array, or `undefined` if the array is empty.
 *
 * @example
 * const emptyArr: number[] = [];
 * const noElement = head(emptyArr);
 * // noElement will be undefined
 */
declare function head<T>(arr: readonly T[]): T | undefined;

export { head };
