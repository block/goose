/**
 * Checks if `value` is a function.
 *
 * @param {any} value The value to check.
 * @returns {value is (...args: any[]) => any} Returns `true` if `value` is a function, else `false`.
 *
 * @example
 * isFunction(Array.prototype.slice); // true
 * isFunction(async function () {}); // true
 * isFunction(function* () {}); // true
 * isFunction(Proxy); // true
 * isFunction(Int8Array); // true
 */
declare function isFunction(value: any): value is (...args: any[]) => any;

export { isFunction };
