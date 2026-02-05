/**
 * Checks if a given value is a native function.
 *
 * This function tests whether the provided value is a native function implemented by the JavaScript engine.
 * It returns `true` if the value is a native function, and `false` otherwise.
 *
 * @param {any} value - The value to test for native function.
 * @returns {value is (...args: any[]) => any} `true` if the value is a native function, `false` otherwise.
 *
 * @example
 * const value1 = Array.prototype.push;
 * const value2 = () => {};
 * const result1 = isNative(value1); // true
 * const result2 = isNative(value2); // false
 */
declare function isNative(value: any): value is (...args: any[]) => any;

export { isNative };
