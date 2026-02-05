/**
 * Checks if a given value is `ArrayBuffer`.
 *
 * This function can also serve as a type predicate in TypeScript, narrowing the type of the argument to `ArrayBuffer`.
 *
 * @param {any} value The value to check if it is a `ArrayBuffer`.
 * @returns {value is ArrayBuffer} Returns `true` if `value` is a `ArrayBuffer`, else `false`.
 *
 * @example
 * const value1 = new ArrayBuffer();
 * const value2 = new Array();
 * const value3 = new Map();
 *
 * console.log(isArrayBuffer(value1)); // true
 * console.log(isArrayBuffer(value2)); // false
 * console.log(isArrayBuffer(value3)); // false
 */
declare function isArrayBuffer(value?: any): value is ArrayBuffer;

export { isArrayBuffer };
