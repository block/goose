interface FilterAsyncOptions {
    concurrency?: number;
}
/**
 * Filters an array asynchronously using an async predicate function.
 *
 * Returns a promise that resolves to a new array containing only the elements
 * for which the predicate function returns a truthy value.
 *
 * @template T - The type of elements in the array.
 * @param {readonly T[]} array The array to filter.
 * @param {(item: T, index: number, array: readonly T[]) => Promise<boolean>} predicate An async function that tests each element.
 * @param {FilterAsyncOptions} [options] Optional configuration object.
 * @param {number} [options.concurrency] Maximum number of concurrent async operations. If not specified, all operations run concurrently.
 * @returns {Promise<T[]>} A promise that resolves to the filtered array.
 * @example
 * const users = [{ id: 1, active: true }, { id: 2, active: false }, { id: 3, active: true }];
 * const activeUsers = await filterAsync(users, async (user) => {
 *   return await checkUserStatus(user.id);
 * });
 * // Returns: [{ id: 1, active: true }, { id: 3, active: true }]
 *
 * @example
 * // With concurrency limit
 * const numbers = [1, 2, 3, 4, 5];
 * const evenNumbers = await filterAsync(
 *   numbers,
 *   async (n) => await isEvenAsync(n),
 *   { concurrency: 2 }
 * );
 * // Processes at most 2 operations concurrently
 */
declare function filterAsync<T>(array: readonly T[], predicate: (item: T, index: number, array: readonly T[]) => Promise<boolean>, options?: FilterAsyncOptions): Promise<T[]>;

export { filterAsync };
