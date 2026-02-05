/**
 * This function retrieves the names of string-keyed properties from an object, including those inherited from its prototype.
 *
 * - If the value is not an object, it is converted to an object.
 * - Array-like objects are treated like arrays.
 * - Sparse arrays with some missing indices are treated like dense arrays.
 * - If the value is `null` or `undefined`, an empty array is returned.
 * - When handling prototype objects, the `constructor` property is excluded from the results.
 *
 * @param {any} [object] - The object to inspect for keys.
 * @returns {string[]} An array of string keys from the object.
 *
 * @example
 * const obj = { a: 1, b: 2 };
 * console.log(keysIn(obj)); // ['a', 'b']
 *
 * const arr = [1, 2, 3];
 * console.log(keysIn(arr)); // ['0', '1', '2']
 *
 * function Foo() {}
 * Foo.prototype.a = 1;
 * console.log(keysIn(new Foo())); // ['a']
 */
declare function keysIn(object?: any): string[];

export { keysIn };
