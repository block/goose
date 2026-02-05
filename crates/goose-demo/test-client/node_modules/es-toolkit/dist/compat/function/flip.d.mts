/**
 * Reverses the order of arguments for a given function.
 *
 * @template T - The type of the function being flipped.
 * @param {T} func - The function whose arguments will be reversed.
 * @returns {T} A new function that takes the reversed arguments and returns the result of calling `func`.
 *
 * @example
 * var flipped = flip(function() {
 *   return Array.prototype.slice.call(arguments);
 * });
 *
 * flipped('a', 'b', 'c', 'd');
 * // => ['d', 'c', 'b', 'a']
 */
declare function flip<T extends (...args: any) => any>(func: T): T;

export { flip };
