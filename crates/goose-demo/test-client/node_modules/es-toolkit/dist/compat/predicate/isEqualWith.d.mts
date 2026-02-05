import { IsEqualCustomizer } from '../_internal/IsEqualCustomizer.mjs';

/**
 * Compares two values for equality using a custom comparison function.
 *
 * The custom function allows for fine-tuned control over the comparison process. If it returns a boolean, that result determines the equality. If it returns undefined, the function falls back to the default equality comparison.
 *
 * This function also uses the custom equality function to compare values inside objects,
 * arrays, maps, sets, and other complex structures, ensuring a deep comparison.
 *
 * This approach provides flexibility in handling complex comparisons while maintaining efficient default behavior for simpler cases.
 *
 * The custom comparison function can take up to six parameters:
 * - `x`: The value from the first object `a`.
 * - `y`: The value from the second object `b`.
 * - `property`: The property key used to get `x` and `y`.
 * - `xParent`: The parent of the first value `x`.
 * - `yParent`: The parent of the second value `y`.
 * - `stack`: An internal stack (Map) to handle circular references.
 *
 * @param {unknown} a - The first value to compare.
 * @param {unknown} b - The second value to compare.
 * @param {(x: any, y: any, property?: PropertyKey, xParent?: any, yParent?: any, stack?: Map<any, any>) => boolean | void} [areValuesEqual=noop] - A function to customize the comparison.
 *   If it returns a boolean, that result will be used. If it returns undefined,
 *   the default equality comparison will be used.
 * @returns {boolean} `true` if the values are equal according to the customizer, otherwise `false`.
 *
 * @example
 * const customizer = (a, b) => {
 *   if (typeof a === 'string' && typeof b === 'string') {
 *     return a.toLowerCase() === b.toLowerCase();
 *   }
 * };
 * isEqualWith('Hello', 'hello', customizer); // true
 * isEqualWith({ a: 'Hello' }, { a: 'hello' }, customizer); // true
 * isEqualWith([1, 2, 3], [1, 2, 3], customizer); // true
 */
declare function isEqualWith(a: any, b: any, areValuesEqual?: IsEqualCustomizer): boolean;

export { isEqualWith };
