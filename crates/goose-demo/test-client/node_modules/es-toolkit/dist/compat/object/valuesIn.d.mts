/**
 * Retrieves the values from an object, including those inherited from its prototype.
 *
 * @template T
 * @param {Record<string, T> | Record<number, T> | ArrayLike<T> | null | undefined} object - The object to query.
 * @returns {T[]} Returns an array of property values.
 *
 * @example
 * const obj = { a: 1, b: 2, c: 3 };
 * valuesIn(obj); // => [1, 2, 3]
 */
declare function valuesIn<T>(object: Record<string, T> | Record<number, T> | ArrayLike<T> | null | undefined): T[];
/**
 * Retrieves the values from an object, including those inherited from its prototype.
 *
 * @template T
 * @param {T | null | undefined} object - The object to query.
 * @returns {Array<T[keyof T]>} Returns an array of property values.
 *
 * @example
 * const obj = { a: 1, b: 2, c: 3 };
 * valuesIn(obj); // => [1, 2, 3]
 */
declare function valuesIn<T extends object>(object: T | null | undefined): Array<T[keyof T]>;

export { valuesIn };
