/**
 * Creates a memoized version of the provided function. The memoized function caches
 * results based on the argument it receives, so if the same argument is passed again,
 * it returns the cached result instead of recomputing it.
 *
 * This function works with functions that take zero or just one argument. If your function
 * originally takes multiple arguments, you should refactor it to take a single object or array
 * that combines those arguments.
 *
 * If the argument is not primitive (e.g., arrays or objects), provide a
 * `getCacheKey` function to generate a unique cache key for proper caching.
 *
 * @template F - The type of the function to be memoized.
 * @param {F} fn - The function to be memoized. It should accept a single argument and return a value.
 * @param {MemoizeOptions<Parameters<F>[0], ReturnType<F>>} [options={}] - Optional configuration for the memoization.
 * @param {MemoizeCache<any, V>} [options.cache] - The cache object used to store results. Defaults to a new `Map`.
 * @param {(args: A) => unknown} [options.getCacheKey] - An optional function to generate a unique cache key for each argument.
 *
 * @returns The memoized function with an additional `cache` property that exposes the internal cache.
 *
 * @example
 * // Example using the default cache
 * const add = (x: number) => x + 10;
 * const memoizedAdd = memoize(add);
 *
 * console.log(memoizedAdd(5)); // 15
 * console.log(memoizedAdd(5)); // 15 (cached result)
 * console.log(memoizedAdd.cache.size); // 1
 *
 * @example
 * // Example using a custom resolver
 * const sum = (arr: number[]) => arr.reduce((x, y) => x + y, 0);
 * const memoizedSum = memoize(sum, { getCacheKey: (arr: number[]) => arr.join(',') });
 * console.log(memoizedSum([1, 2])); // 3
 * console.log(memoizedSum([1, 2])); // 3 (cached result)
 * console.log(memoizedSum.cache.size); // 1
 *
 * @example
 * // Example using a custom cache implementation
 * class CustomCache<K, T> implements MemoizeCache<K, T> {
 *   private cache = new Map<K, T>();
 *
 *   set(key: K, value: T): void {
 *     this.cache.set(key, value);
 *   }
 *
 *   get(key: K): T | undefined {
 *     return this.cache.get(key);
 *   }
 *
 *   has(key: K): boolean {
 *     return this.cache.has(key);
 *   }
 *
 *   delete(key: K): boolean {
 *     return this.cache.delete(key);
 *   }
 *
 *   clear(): void {
 *     this.cache.clear();
 *   }
 *
 *   get size(): number {
 *     return this.cache.size;
 *   }
 * }
 * const customCache = new CustomCache<string, number>();
 * const memoizedSumWithCustomCache = memoize(sum, { cache: customCache });
 * console.log(memoizedSumWithCustomCache([1, 2])); // 3
 * console.log(memoizedSumWithCustomCache([1, 2])); // 3 (cached result)
 * console.log(memoizedAddWithCustomCache.cache.size); // 1
 */
declare function memoize<F extends (...args: any) => any>(fn: F, options?: {
    cache?: MemoizeCache<any, ReturnType<F>>;
    getCacheKey?: (args: Parameters<F>[0]) => unknown;
}): F & {
    cache: MemoizeCache<any, ReturnType<F>>;
};
/**
 * Represents a cache for memoization, allowing storage and retrieval of computed values.
 *
 * @template K - The type of keys used to store values in the cache.
 * @template V - The type of values stored in the cache.
 */
interface MemoizeCache<K, V> {
    /**
     * Stores a value in the cache with the specified key.
     *
     * @param key - The key to associate with the value.
     * @param value - The value to store in the cache.
     */
    set(key: K, value: V): void;
    /**
     * Retrieves a value from the cache by its key.
     *
     * @param key - The key of the value to retrieve.
     * @returns The value associated with the key, or undefined if the key does not exist.
     */
    get(key: K): V | undefined;
    /**
     * Checks if a value exists in the cache for the specified key.
     *
     * @param key - The key to check for existence in the cache.
     * @returns True if the cache contains the key, false otherwise.
     */
    has(key: K): boolean;
    /**
     * Deletes a value from the cache by its key.
     *
     * @param key - The key of the value to delete.
     * @returns True if the value was successfully deleted, false otherwise.
     */
    delete(key: K): boolean | void;
    /**
     * Clears all values from the cache.
     */
    clear(): void;
    /**
     * The number of entries in the cache.
     */
    size: number;
}

export { type MemoizeCache, memoize };
