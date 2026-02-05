/**
 * Subtracts one number from another.
 *
 * If either of the numbers is `NaN`, the function returns `NaN`.
 *
 * @param {number} value The first number. (minuend)
 * @param {number} other The second number.(subtrahend)
 * @returns {number} The difference of the two numbers, or `NaN` if any input is `NaN`.
 *
 * @example
 * subtract(6, 3); // => 3
 * subtract(6, NaN); // => NaN
 * subtract(NaN, 3); // => NaN
 */
declare function subtract(value: number, other: number): number;

export { subtract };
