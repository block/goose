/**
 * Gets a random element from collection.
 *
 * @template T
 * @param {readonly [T, ...T[]]} collection - The collection to sample.
 * @returns {T} Returns the random element.
 *
 * @example
 * sample([1, 2, 3, 4]);
 * // => 2
 */
declare function sample<T>(collection: readonly [T, ...T[]]): T;
/**
 * Gets a random element from collection.
 *
 * @template T
 * @param {Record<string, T> | Record<number, T> | null | undefined} collection - The collection to sample.
 * @returns {T | undefined} Returns the random element.
 *
 * @example
 * sample({ 'a': 1, 'b': 2, 'c': 3 });
 * // => 2
 */
declare function sample<T>(collection: Record<string, T> | Record<number, T> | null | undefined): T | undefined;
/**
 * Gets a random element from collection.
 *
 * @template T
 * @param {T | null | undefined} collection - The collection to sample.
 * @returns {T[keyof T] | undefined} Returns the random element.
 *
 * @example
 * sample({ 'a': 1, 'b': 2, 'c': 3 });
 * // => 2
 */
declare function sample<T extends object>(collection: T | null | undefined): T[keyof T] | undefined;

export { sample };
