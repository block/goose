/**
 * Divide two numbers.
 *
 * If either of the numbers is `NaN`, the function returns `NaN`.
 *
 * @param {number} value The first number in a division.
 * @param {number} other The second number in a division.
 * @returns {number} The quotient of value and other.
 *
 * @example
 * divide(6, 3); // => 2
 * divide(2, NaN); // => NaN
 * divide(NaN, 3); // => NaN
 * divide(NaN, NaN); // => NaN
 */
declare function divide(value: number, other: number): number;

export { divide };
