/**
 * Checks if `value` is a finite number.
 *
 * Acts as a type guard for `number` values — returning `true` only when `value`
 * is of type `number` and finite (not `Infinity`, `-Infinity`, or `NaN`).
 *
 * @param {unknown} value The value to check.
 * @returns {value is number} Returns `true` if `value` is a finite number, `false` otherwise.
 *
 * @example
 * ```typescript
 * const value1 = 100;
 * const value2 = Infinity;
 * const value3 = '100';
 *
 * console.log(isFinite(value1)); // true
 * console.log(isFinite(value2)); // false
 * console.log(isFinite(value3)); // false
 *
 * if (isFinite(value1)) {
 *   console.log(value1.toFixed(2));
 * }
 * ```
 */
declare function isFinite(value: unknown): value is number;

export { isFinite };
