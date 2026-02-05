/**
 * Creates a new object with specified keys omitted.
 *
 * This function takes an object and an array of keys, and returns a new object that
 * excludes the properties corresponding to the specified keys.
 *
 * @template T - The type of object.
 * @template K - The type of keys in object.
 * @param {T} obj - The object to omit keys from.
 * @param {K[]} keys - An array of keys to be omitted from the object.
 * @returns {Omit<T, K>} A new object with the specified keys omitted.
 *
 * @example
 * const obj = { a: 1, b: 2, c: 3 };
 * const result = omit(obj, ['b', 'c']);
 * // result will be { a: 1 }
 */
declare function omit<T extends Record<PropertyKey, any>, K extends keyof T>(obj: T, keys: readonly K[]): Omit<T, K>;
/**
 * Creates a new object with specified keys omitted.
 *
 * This overload supports dynamic key arrays determined at runtime,
 * useful when working with keys from Object.keys() or similar operations.
 *
 * @template T - The type of object.
 * @param {T} obj - The object to omit keys from.
 * @param {PropertyKey[]} keys - An array of keys to be omitted from the object. Supports dynamic arrays.
 * @returns {Partial<T>} A new object with the specified keys omitted.
 *
 * @example
 * const obj = { a: 1, b: 2, c: 3 };
 * const keysToOmit = Object.keys({ b: true, c: true }); // string[]
 * const result = omit(obj, keysToOmit);
 * // result will be { a: 1 }
 */
declare function omit<T extends Record<PropertyKey, any>>(obj: T, keys: readonly PropertyKey[]): Partial<T>;

export { omit };
