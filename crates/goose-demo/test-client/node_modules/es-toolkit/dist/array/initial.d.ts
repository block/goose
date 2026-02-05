/**
 * Returns an empty array when the input is a tuple containing exactly one element.
 *
 * @template T The type of the single element.
 * @param {[T]} arr - A tuple containing exactly one element.
 * @returns {[]} An empty array since there is only one element.
 *
 * @example
 * const array = [100] as const;
 * const result = initial(array);
 * // result will be []
 */
declare function initial<T>(arr: readonly [T]): [];
/**
 * Returns an empty array when the input array is empty.
 *
 * @returns {[]} Always returns an empty array for an empty input.
 *
 * @example
 * const array = [] as const;
 * const result = initial(array);
 * // result will be []
 */
declare function initial(arr: readonly []): [];
/**
 * Returns a new array containing all elements except the last one from a tuple with multiple elements.
 *
 * @template T The types of the initial elements.
 * @template U The type of the last element in the tuple.
 * @param {[...T[], U]} arr - A tuple with one or more elements.
 * @returns {T[]} A new array containing all but the last element of the tuple.
 *
 * @example
 * const array = ['apple', 'banana', 'cherry'] as const;
 * const result = initial(array);
 * // result will be ['apple', 'banana']
 */
declare function initial<T, U>(arr: readonly [...T[], U]): T[];
/**
 * Returns a new array containing all elements except the last one from the input array.
 * If the input array is empty or has only one element, the function returns an empty array.
 *
 * @template T The type of elements in the array.
 * @param {T[]} arr - The input array.
 * @returns {T[]} A new array containing all but the last element of the input array.
 *
 * @example
 * const arr = [1, 2, 3, 4];
 * const result = initial(arr);
 * // result will be [1, 2, 3]
 */
declare function initial<T>(arr: readonly T[]): T[];

export { initial };
