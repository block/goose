interface MapCache {
    /**
     * Removes the value associated with the specified key from the cache.
     *
     * @param key - The key of the value to remove
     * @returns `true` if an element was removed, `false` if the key wasn't found
     *
     * @example
     * ```typescript
     * cache.set('user', { id: 123, name: 'John' });
     * cache.delete('user'); // Returns true
     * cache.delete('unknown'); // Returns false
     * ```
     */
    delete(key: any): boolean;
    /**
     * Retrieves the value associated with the specified key from the cache.
     *
     * @param key - The key of the value to retrieve
     * @returns The cached value or undefined if not found
     *
     * @example
     * ```typescript
     * cache.set('user', { id: 123, name: 'John' });
     * cache.get('user'); // Returns { id: 123, name: 'John' }
     * cache.get('unknown'); // Returns undefined
     * ```
     */
    get(key: any): any;
    /**
     * Checks if the cache contains a value for the specified key.
     *
     * @param key - The key to check for existence
     * @returns `true` if the key exists in the cache, otherwise `false`
     *
     * @example
     * ```typescript
     * cache.set('user', { id: 123, name: 'John' });
     * cache.has('user'); // Returns true
     * cache.has('unknown'); // Returns false
     * ```
     */
    has(key: any): boolean;
    /**
     * Stores a value in the cache with the specified key.
     * If the key already exists, its value is updated.
     *
     * @param key - The key to associate with the value
     * @param value - The value to store in the cache
     * @returns The cache instance for method chaining
     *
     * @example
     * ```typescript
     * cache.set('user', { id: 123, name: 'John' })
     *      .set('settings', { theme: 'dark' });
     * ```
     */
    set(key: any, value: any): this;
    /**
     * Removes all key-value pairs from the cache.
     * This method is optional as some cache implementations may be immutable.
     *
     * @example
     * ```typescript
     * cache.set('user', { id: 123, name: 'John' });
     * cache.set('settings', { theme: 'dark' });
     * cache.clear(); // Cache is now empty
     * ```
     */
    clear?(): void;
}
/**
 * Constructor interface for creating a new MapCache instance.
 * This defines the shape of a constructor that can create cache objects
 * conforming to the MapCache interface.
 *
 * @example
 * ```typescript
 * class CustomCache implements MapCache {
 *   // Cache implementation
 * }
 *
 * const CacheConstructor: MapCacheConstructor = CustomCache;
 * const cache = new CacheConstructor();
 * ```
 */
interface MapCacheConstructor {
    new (): MapCache;
}
/**
 * Represents a function that has been memoized.
 * A memoized function maintains the same signature as the original function
 * but adds a cache property to store previously computed results.
 *
 * @template T - The type of the original function being memoized
 */
interface MemoizedFunction {
    /**
     * The cache storing previously computed results
     */
    cache: MapCache;
}
/**
 * Creates a function that memoizes the result of func. If resolver is provided it determines the cache key for
 * storing the result based on the arguments provided to the memoized function. By default, the first argument
 * provided to the memoized function is coerced to a string and used as the cache key. The func is invoked with
 * the this binding of the memoized function.
 *
 * @template T - The type of the original function being memoized
 * @param {T} func The function to have its output memoized.
 * @param {Function} [resolver] The function to resolve the cache key.
 * @return {MemoizedFunction<T>} Returns the new memoizing function.
 */
declare function memoize<T extends (...args: any) => any>(func: T, resolver?: (...args: Parameters<T>) => any): T & MemoizedFunction;
declare namespace memoize {
    var Cache: MapCacheConstructor;
}

export { memoize };
