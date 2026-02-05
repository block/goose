/**
 * Invokes the iteratee function n times, returning an array of the results.
 *
 * @template T The return type of the iteratee function.
 * @param {number} n - The number of times to invoke iteratee.
 * @param {(num: number) => T} iteratee - The function to invoke for each index.
 * @returns {T[]} An array containing the results of invoking iteratee n times.
 * @example
 * times(3, (i) => i * 2); // => [0, 2, 4]
 * times(2, () => 'es-toolkit'); // => ['es-toolkit', 'es-toolkit']
 */
declare function times<T>(n: number, iteratee: (num: number) => T): T[];
/**
 * Invokes the default iteratee function n times, returning an array of indices.
 *
 * @param {number} n - The number of times to invoke the default iteratee.
 * @returns {number[]} An array containing indices from 0 to n-1.
 * @example
 * times(3); // => [0, 1, 2]
 */
declare function times(n: number): number[];

export { times };
