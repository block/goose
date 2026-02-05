/**
 * Multiply two numbers.
 *
 * If either of the numbers is `NaN`, the function returns `NaN`.
 *
 * @param {number} value The first number in a multiplication
 * @param {number} other The second number in a multiplication
 * @returns {number} The product of value and other
 *
 * @example
 * multiply(2, 3); // => 6
 * multiply(2, NaN); // => NaN
 * multiply(NaN, 3); // => NaN
 * multiply(NaN, NaN); // => NaN
 */
declare function multiply(value: number, other: number): number;

export { multiply };
