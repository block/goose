/**
 * Invokes the method at path of each element in collection.
 *
 * @param {object | null | undefined} collection - The collection to iterate over.
 * @param {string} methodName - The name of the method to invoke.
 * @param {...any[]} args - The arguments to invoke each method with.
 * @returns {any[]} Returns the array of results.
 *
 * @example
 * invokeMap([[5, 1, 7], [3, 2, 1]], 'sort');
 * // => [[1, 5, 7], [1, 2, 3]]
 *
 * invokeMap([123, 456], 'toString', 2);
 * // => ['1111011', '111001000']
 */
declare function invokeMap(collection: object | null | undefined, methodName: string, ...args: any[]): any[];
/**
 * Invokes the method at path of each element in collection.
 *
 * @template R
 * @param {object | null | undefined} collection - The collection to iterate over.
 * @param {(...args: any[]) => R} method - The method to invoke.
 * @param {...any[]} args - The arguments to invoke each method with.
 * @returns {R[]} Returns the array of results.
 *
 * @example
 * invokeMap([5, 1, 7], Array.prototype.slice, 1);
 * // => [[], [], []]
 */
declare function invokeMap<R>(collection: object | null | undefined, method: (...args: any[]) => R, ...args: any[]): R[];

export { invokeMap };
