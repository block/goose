/**
 * Checks if a specified value exists within a given array-like collection.
 *
 * The comparison uses SameValueZero to check for inclusion.
 *
 * @template T The type of elements in the collection
 * @param collection The array-like collection to search in
 * @param target The value to search for in the collection
 * @param [fromIndex=0] The index to start searching from. If negative, it is treated as an offset from the end
 * @returns `true` if the value is found in the collection, `false` otherwise
 *
 * @example
 * includes([1, 2, 3], 2); // true
 * includes([1, 2, 3], 4); // false
 * includes('hello', 'e'); // true
 * includes(null, 1); // false
 * includes([1, 2, 3], 2, 2); // false
 * includes([1, 2, 3], 2, -2); // true
 */
declare function includes<T>(collection: Record<string, T> | Record<number, T> | null | undefined, target: T, fromIndex?: number): boolean;

export { includes };
