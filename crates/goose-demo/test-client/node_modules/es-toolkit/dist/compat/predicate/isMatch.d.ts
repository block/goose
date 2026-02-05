/**
 * Checks if the target matches the source by comparing their structures and values.
 * This function supports deep comparison for objects, arrays, maps, and sets.
 *
 * @param {object} target - The target value to match against.
 * @param {object} source - The source value to match with.
 * @returns {boolean} - Returns `true` if the target matches the source, otherwise `false`.
 *
 * @example
 * // Basic usage
 * isMatch({ a: 1, b: 2 }, { a: 1 }); // true
 *
 * @example
 * // Matching arrays
 * isMatch([1, 2, 3], [1, 2, 3]); // true
 *
 * @example
 * // Matching maps
 * const targetMap = new Map([['key1', 'value1'], ['key2', 'value2']]);
 * const sourceMap = new Map([['key1', 'value1']]);
 * isMatch(targetMap, sourceMap); // true
 *
 * @example
 * // Matching sets
 * const targetSet = new Set([1, 2, 3]);
 * const sourceSet = new Set([1, 2]);
 * isMatch(targetSet, sourceSet); // true
 */
declare function isMatch(target: object, source: object): boolean;

export { isMatch };
