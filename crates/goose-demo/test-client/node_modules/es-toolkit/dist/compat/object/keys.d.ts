/**
 * Creates an array of the own enumerable property names of `object`.
 *
 * Non-object values are coerced to objects.
 *
 * @param {object} object The object to query.
 * @returns {string[]} Returns the array of property names.
 * @example
 * function Foo() {
 *   this.a = 1;
 *   this.b = 2;
 * }
 * Foo.prototype.c = 3;
 * keys(new Foo); // ['a', 'b'] (iteration order is not guaranteed)
 *
 * keys('hi'); // ['0', '1']
 * keys([1, 2, 3]); // ['0', '1', '2']
 * keys({ a: 1, b: 2 }); // ['a', 'b']
 */
declare function keys(object?: any): string[];

export { keys };
