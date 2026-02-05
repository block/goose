/**
 * Creates a function that invokes the method at a given path of `object` with the provided arguments.
 *
 * @param {object} object - The object to query.
 * @param {...any} args - The arguments to invoke the method with.
 * @returns {(path: PropertyKey | PropertyKey[]) => any} - Returns a new function that takes a path and invokes the method at `path` with `args`.
 *
 * @example
 * const object = {
 *  a: {
 *   b: function (x, y) {
 *    return x + y;
 *    }
 *   }
 * };
 *
 * const add = methodOf(object, 1, 2);
 * console.log(add('a.b')); // => 3
 */
declare function methodOf(object: object, ...args: any[]): (path: PropertyKey | readonly PropertyKey[]) => any;

export { methodOf };
