/**
 * Returns the input value unchanged.
 *
 * @template T - The type of the input value.
 * @param {T} x - The value to be returned.
 * @returns {T} The input value.
 *
 * @example
 * // Returns 5
 * identity(5);
 *
 * @example
 * // Returns 'hello'
 * identity('hello');
 *
 * @example
 * // Returns { key: 'value' }
 * identity({ key: 'value' });
 */
declare function identity<T>(x: T): T;

export { identity };
