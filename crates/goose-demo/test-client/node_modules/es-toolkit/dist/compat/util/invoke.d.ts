/**
 * Invokes the method at `path` of `object` with the given arguments.
 *
 * @param {any} object - The object to query.
 * @param {PropertyKey | PropertyKey[]} path - The path of the method to invoke.
 * @param {any[]} args - The arguments to invoke the method with.
 * @returns {any} - Returns the result of the invoked method.
 *
 * @example
 * const object = {
 *   a: {
 *     b: function (x, y) {
 *       return x + y;
 *     }
 *   }
 * };
 *
 * invoke(object, 'a.b', [1, 2]); // => 3
 * invoke(object, ['a', 'b'], [1, 2]); // => 3
 */
declare function invoke(object: any, path: PropertyKey | readonly PropertyKey[], ...args: any[]): any;

export { invoke };
