import { Many } from '../_internal/Many.js';

/**
 * Creates a function that invokes `func` with arguments arranged according to the specified `indices`
 * where the argument value at the first index is provided as the first argument,
 * the argument value at the second index is provided as the second argument, and so on.
 *
 * @param {(...args: any[]) => any} func The function to rearrange arguments for.
 * @param {Array<number | number[]>} indices The arranged argument indices.
 * @returns {(...args: any[]) => any} Returns the new function.
 *
 * @example
 * const greet = (greeting: string, name: string) => `${greeting}, ${name}!`;
 * const rearrangedGreet = rearg(greet, 1, 0);
 * console.log(rearrangedGreet('World', 'Hello')); // Output: "Hello, World!"
 */
declare function rearg(func: (...args: any[]) => any, ...indices: Array<Many<number>>): (...args: any[]) => any;

export { rearg };
