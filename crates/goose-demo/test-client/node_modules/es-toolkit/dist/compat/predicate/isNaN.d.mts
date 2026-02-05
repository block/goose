/**
 * Checks if the value is NaN.
 *
 * @param {any} value - The value to check.
 * @returns {boolean} `true` if the value is NaN, `false` otherwise.
 *
 * @example
 * isNaN(NaN); // true
 * isNaN(0); // false
 * isNaN('NaN'); // false
 * isNaN(undefined); // false
 */
declare function isNaN(value?: any): boolean;

export { isNaN };
