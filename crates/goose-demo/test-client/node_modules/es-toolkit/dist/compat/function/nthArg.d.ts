/**
 * Creates a function that gets the argument at index `n`. If `n` is negative, the nth argument from the end is returned.
 *
 * @param {number} [n=0] - The index of the argument to return.
 * @returns {(...args: any[]) => any} Returns the new function.
 *
 * @example
 * var func = nthArg(1);
 * func('a', 'b', 'c', 'd');
 * // => 'b'
 *
 * var func = nthArg(-2);
 * func('a', 'b', 'c', 'd');
 * // => 'c'
 */
declare function nthArg(n?: number): (...args: any[]) => any;

export { nthArg };
