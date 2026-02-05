/**
 * Removes the property at the given path of the object.
 *
 * @param {unknown} obj - The object to modify.
 * @param {PropertyKey | readonly PropertyKey[]} path - The path of the property to unset.
 * @returns {boolean} - Returns true if the property is deleted, else false.
 *
 * @example
 * const obj = { a: { b: { c: 42 } } };
 * unset(obj, 'a.b.c'); // true
 * console.log(obj); // { a: { b: {} } }
 *
 * @example
 * const obj = { a: { b: { c: 42 } } };
 * unset(obj, ['a', 'b', 'c']); // true
 * console.log(obj); // { a: { b: {} } }
 */
declare function unset(obj: any, path: PropertyKey | readonly PropertyKey[]): boolean;

export { unset };
