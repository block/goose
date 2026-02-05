/**
 * Iterates over an object and invokes the `iteratee` function for each property.
 *
 * Iterates over string keyed properties including inherited properties.
 *
 * The iteration is terminated early if the `iteratee` function returns `false`.
 *
 * @template T - The type of the object
 * @param {T} object - The object to iterate over
 * @param {(value: T[keyof T], key: string, obj: T) => any} iteratee - The function invoked per iteration
 * @returns {T} Returns the object
 *
 * @example
 * // Iterate over all properties including inherited ones
 * const obj = { a: 1, b: 2 };
 * forIn(obj, (value, key) => {
 *   console.log(key, value);
 * });
 * // Output: 'a' 1, 'b' 2
 *
 * // Early termination
 * forIn(obj, (value, key) => {
 *   console.log(key, value);
 *   return key !== 'a'; // stop after 'a'
 * });
 * // Output: 'a' 1
 */
declare function forIn<T>(object: T, iteratee?: (value: T[keyof T], key: string, collection: T) => any): T;
/**
 * Iterates over an object and invokes the `iteratee` function for each property.
 *
 * Iterates over string keyed properties including inherited properties.
 *
 * The iteration is terminated early if the `iteratee` function returns `false`.
 *
 * @template T - The type of the object
 * @param {T | null | undefined} object - The object to iterate over
 * @param {(value: T[keyof T], key: string, obj: T) => any} iteratee - The function invoked per iteration
 * @returns {T | null | undefined} Returns the object
 *
 * @example
 * // Iterate over all properties including inherited ones
 * const obj = { a: 1, b: 2 };
 * forIn(obj, (value, key) => {
 *   console.log(key, value);
 * });
 * // Output: 'a' 1, 'b' 2
 *
 * // Early termination
 * forIn(obj, (value, key) => {
 *   console.log(key, value);
 *   return key !== 'a'; // stop after 'a'
 * });
 * // Output: 'a' 1
 */
declare function forIn<T>(object: T | null | undefined, iteratee?: (value: T[keyof T], key: string, collection: T) => any): T | null | undefined;

export { forIn };
