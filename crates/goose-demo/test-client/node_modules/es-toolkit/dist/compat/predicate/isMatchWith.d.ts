import { IsMatchWithCustomizer } from '../_internal/IsMatchWithCustomizer.js';

/**
 * Performs a deep comparison between a target value and a source pattern to determine if they match,
 * using a custom comparison function for fine-grained control over the matching logic.
 *
 * @param {object} target - The value to be tested for matching
 * @param {object} source - The pattern/template to match against
 * @param {IsMatchWithCustomizer} compare - Custom comparison function for fine-grained control
 * @returns {boolean} `true` if the target matches the source pattern, `false` otherwise
 *
 * @example
 * // Basic matching with custom comparator
 * const caseInsensitiveCompare = (objVal, srcVal) => {
 *   if (typeof objVal === 'string' && typeof srcVal === 'string') {
 *     return objVal.toLowerCase() === srcVal.toLowerCase();
 *   }
 *   return undefined;
 * };
 *
 * isMatchWith(
 *   { name: 'JOHN', age: 30 },
 *   { name: 'john' },
 *   caseInsensitiveCompare
 * ); // true
 */
declare function isMatchWith(target: object, source: object, compare: IsMatchWithCustomizer): boolean;

export { isMatchWith };
