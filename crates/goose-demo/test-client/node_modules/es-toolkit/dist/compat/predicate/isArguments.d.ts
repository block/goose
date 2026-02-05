/**
 * Checks if the given value is an arguments object.
 *
 * This function tests whether the provided value is an arguments object or not.
 * It returns `true` if the value is an arguments object, and `false` otherwise.
 *
 * This function can also serve as a type predicate in TypeScript, narrowing the type of the argument to an arguments object.
 *
 * @param {any} value - The value to test if it is an arguments object.
 * @returns {value is IArguments} `true` if the value is an arguments, `false` otherwise.
 *
 * @example
 * const args = (function() { return arguments; })();
 * const strictArgs = (function() { 'use strict'; return arguments; })();
 * const value = [1, 2, 3];
 *
 * console.log(isArguments(args)); // true
 * console.log(isArguments(strictArgs)); // true
 * console.log(isArguments(value)); // false
 */
declare function isArguments(value?: any): value is IArguments;

export { isArguments };
