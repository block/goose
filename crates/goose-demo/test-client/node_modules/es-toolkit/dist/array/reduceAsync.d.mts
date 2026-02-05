/**
 * Reduces an array to a single value using an async reducer function.
 *
 * Applies the reducer function sequentially to each element (left to right),
 * carrying an accumulated result from one call to the next. Unlike other async
 * array methods, reduce must process elements sequentially and does not support
 * concurrency limiting.
 *
 * @template T - The type of elements in the array.
 * @template U - The type of the accumulated result.
 * @param {readonly T[]} array The array to reduce.
 * @param {(accumulator: U, currentValue: T, currentIndex: number, array: readonly T[]) => Promise<U>} reducer An async function that processes each element.
 * @param {U} initialValue The initial value of the accumulator.
 * @returns {Promise<U>} A promise that resolves to the final accumulated value.
 * @example
 * const numbers = [1, 2, 3, 4, 5];
 * const sum = await reduceAsync(
 *   numbers,
 *   async (acc, n) => acc + await fetchValue(n),
 *   0
 * );
 * // Returns: sum of all fetched values
 *
 * @example
 * const users = [{ id: 1 }, { id: 2 }, { id: 3 }];
 * const userMap = await reduceAsync(
 *   users,
 *   async (acc, user) => {
 *     const details = await fetchUserDetails(user.id);
 *     acc[user.id] = details;
 *     return acc;
 *   },
 *   {} as Record<number, any>
 * );
 * // Returns: { 1: {...}, 2: {...}, 3: {...} }
 */
declare function reduceAsync<T, U>(array: readonly T[], reducer: (accumulator: U, currentValue: T, currentIndex: number, array: readonly T[]) => Promise<U>, initialValue: U): Promise<U>;
/**
 * Reduces an array to a single value using an async reducer function.
 *
 * Applies the reducer function sequentially to each element (left to right),
 * carrying an accumulated result from one call to the next. Unlike other async
 * array methods, reduce must process elements sequentially and does not support
 * concurrency limiting.
 *
 * When no initial value is provided, the first element of the array is used as
 * the initial value and the reduction starts from the second element.
 *
 * @template T - The type of elements in the array.
 * @param {readonly T[]} array The array to reduce.
 * @param {(accumulator: T, currentValue: T, currentIndex: number, array: readonly T[]) => Promise<T>} reducer An async function that processes each element.
 * @returns {Promise<T | undefined>} A promise that resolves to the final accumulated value, or undefined if the array is empty.
 * @example
 * const numbers = [1, 2, 3, 4, 5];
 * const sum = await reduceAsync(
 *   numbers,
 *   async (acc, n) => acc + n
 * );
 * // Returns: 15
 *
 * @example
 * const emptyArray: number[] = [];
 * const result = await reduceAsync(
 *   emptyArray,
 *   async (acc, n) => acc + n
 * );
 * // Returns: undefined
 */
declare function reduceAsync<T>(array: readonly T[], reducer: (accumulator: T, currentValue: T, currentIndex: number, array: readonly T[]) => Promise<T>): Promise<T>;

export { reduceAsync };
