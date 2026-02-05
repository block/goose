interface FlatMapAsyncOptions {
    concurrency?: number;
}
/**
 * Maps each element in an array using an async callback function and flattens the result by one level.
 *
 * This is equivalent to calling `mapAsync` followed by `flat(1)`, but more efficient.
 * Each callback should return an array, and all returned arrays are concatenated into
 * a single output array.
 *
 * @template T - The type of elements in the input array.
 * @template R - The type of elements in the arrays returned by the callback.
 * @param {readonly T[]} array The array to transform.
 * @param {(item: T, index: number, array: readonly T[]) => Promise<R[]>} callback An async function that transforms each element into an array.
 * @param {FlatMapAsyncOptions} [options] Optional configuration object.
 * @param {number} [options.concurrency] Maximum number of concurrent async operations. If not specified, all operations run concurrently.
 * @returns {Promise<R[]>} A promise that resolves to a flattened array of transformed values.
 * @example
 * const users = [{ id: 1 }, { id: 2 }];
 * const allPosts = await flatMapAsync(users, async (user) => {
 *   return await fetchUserPosts(user.id);
 * });
 * // Returns: [post1, post2, post3, ...] (all posts from all users)
 *
 * @example
 * // With concurrency limit
 * const numbers = [1, 2, 3];
 * const results = await flatMapAsync(
 *   numbers,
 *   async (n) => await fetchRelatedItems(n),
 *   { concurrency: 2 }
 * );
 * // Processes at most 2 operations concurrently
 */
declare function flatMapAsync<T, R>(array: readonly T[], callback: (item: T, index: number, array: readonly T[]) => Promise<R[]>, options?: FlatMapAsyncOptions): Promise<R[]>;

export { flatMapAsync };
