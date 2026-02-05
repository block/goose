/**
 * Finds the first key in a Map for which the predicate function returns true.
 *
 * This function iterates through the entries of the Map and returns the key of the first
 * entry for which the predicate function returns true. If no entry satisfies the predicate,
 * it returns undefined.
 *
 * @template K - The type of keys in the Map.
 * @template V - The type of values in the Map.
 * @param {Map<K, V>} map - The Map to search.
 * @param {(value: V, key: K, map: Map<K, V>) => boolean} doesMatch - A predicate function that tests each entry.
 * @returns {K | undefined} The key of the first entry that satisfies the predicate, or undefined if none found.
 *
 * @example
 * const map = new Map([
 *   ['apple', { color: 'red', quantity: 10 }],
 *   ['banana', { color: 'yellow', quantity: 5 }],
 *   ['grape', { color: 'purple', quantity: 15 }]
 * ]);
 * const result = findKey(map, (value) => value.quantity > 10);
 * // result will be: 'grape'
 */
declare function findKey<K, V>(map: Map<K, V>, doesMatch: (value: V, key: K, map: Map<K, V>) => boolean): K | undefined;

export { findKey };
