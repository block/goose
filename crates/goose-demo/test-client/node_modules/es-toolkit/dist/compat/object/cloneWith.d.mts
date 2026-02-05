type CloneWithCustomizer<T, R> = (value: T, key: number | string | undefined, object: any, stack: any) => R;
/**
 * Creates a shallow clone of a value with customizer that returns a specific result type.
 *
 * @template T - The type of the value to clone.
 * @template R - The result type extending primitive types or objects.
 * @param {T} value - The value to clone.
 * @param {CloneWithCustomizer<T, R>} customizer - The function to customize cloning.
 * @returns {R} Returns the cloned value.
 *
 * @example
 * const obj = { a: 1, b: 2 };
 * const cloned = cloneWith(obj, (value) => {
 *   if (typeof value === 'object') {
 *     return JSON.parse(JSON.stringify(value));
 *   }
 * });
 * // => { a: 1, b: 2 }
 */
declare function cloneWith<T, R extends object | string | number | boolean | null>(value: T, customizer: CloneWithCustomizer<T, R>): R;
/**
 * Creates a shallow clone of a value with optional customizer.
 *
 * @template T - The type of the value to clone.
 * @template R - The result type.
 * @param {T} value - The value to clone.
 * @param {CloneWithCustomizer<T, R | undefined>} customizer - The function to customize cloning.
 * @returns {R | T} Returns the cloned value or the customized result.
 *
 * @example
 * const obj = { a: 1, b: 2 };
 * const cloned = cloneWith(obj, (value) => {
 *   if (typeof value === 'number') {
 *     return value * 2;
 *   }
 * });
 * // => { a: 2, b: 4 }
 */
declare function cloneWith<T, R>(value: T, customizer: CloneWithCustomizer<T, R | undefined>): R | T;
/**
 * Creates a shallow clone of a value.
 *
 * @template T - The type of the value to clone.
 * @param {T} value - The value to clone.
 * @returns {T} Returns the cloned value.
 *
 * @example
 * const obj = { a: 1, b: 2 };
 * const cloned = cloneWith(obj);
 * // => { a: 1, b: 2 }
 */
declare function cloneWith<T>(value: T): T;

export { cloneWith };
