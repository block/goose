/**
 * Creates an array of key-value pairs from an object, including inherited properties.
 *
 * @template T
 * @param {Record<string, T> | Record<number, T>} object - The object to query.
 * @returns {Array<[string, T]>} Returns the array of key-value pairs.
 *
 * @example
 * const object = { a: 1, b: 2 };
 * toPairsIn(object); // [['a', 1], ['b', 2]]
 */
declare function toPairsIn<T>(object?: Record<string, T> | Record<number, T>): Array<[string, T]>;
/**
 * Creates an array of key-value pairs from an object, including inherited properties.
 *
 * @param {object} object - The object to query.
 * @returns {Array<[string, any]>} Returns the array of key-value pairs.
 *
 * @example
 * const object = { a: 1, b: 2 };
 * toPairsIn(object); // [['a', 1], ['b', 2]]
 */
declare function toPairsIn(object?: object): Array<[string, any]>;

export { toPairsIn };
