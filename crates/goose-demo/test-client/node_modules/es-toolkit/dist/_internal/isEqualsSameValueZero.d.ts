/**
 * Performs a `SameValueZero` comparison between two values to determine if they are equivalent.
 *
 * @param {any} value - The value to compare.
 * @param {any} other - The other value to compare.
 * @returns {boolean} Returns `true` if the values are equivalent, else `false`.
 *
 * @example
 * eq(1, 1); // true
 * eq(0, -0); // true
 * eq(NaN, NaN); // true
 * eq('a', Object('a')); // false
 */
declare function isEqualsSameValueZero(value: any, other: any): boolean;

export { isEqualsSameValueZero };
