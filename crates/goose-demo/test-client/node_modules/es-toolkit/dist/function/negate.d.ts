/**
 * Creates a function that negates the result of the predicate function.
 *
 * @template F - The type of the function to negate.
 * @param {F} func - The function to negate.
 * @returns {F} The new negated function, which negates the boolean result of `func`.
 *
 * @example
 * const array = [1, 2, 3, 4, 5, 6];
 * const isEven = (n: number) => n % 2 === 0;
 * const result = array.filter(negate(isEven));
 * // result will be [1, 3, 5]
 */
declare function negate<F extends (...args: any[]) => boolean>(func: F): F;

export { negate };
