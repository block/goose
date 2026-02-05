/**
 * Checks if a given value is a plain object.
 *
 * A plain object is an object created by the `{}` literal, `new Object()`, or
 * `Object.create(null)`.
 *
 * This function also handles objects with custom
 * `Symbol.toStringTag` properties.
 *
 * `Symbol.toStringTag` is a built-in symbol that a constructor can use to customize the
 * default string description of objects.
 *
 * @param {any} [object] - The value to check.
 * @returns {boolean} - True if the value is a plain object, otherwise false.
 *
 * @example
 * console.log(isPlainObject({})); // true
 * console.log(isPlainObject([])); // false
 * console.log(isPlainObject(null)); // false
 * console.log(isPlainObject(Object.create(null))); // true
 * console.log(isPlainObject(new Map())); // false
 */
declare function isPlainObject(object?: any): boolean;

export { isPlainObject };
