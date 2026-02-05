/**
 * Tests whether all entries in a Map satisfy the provided predicate function.
 *
 * This function iterates through all entries of the Map and checks if the predicate function
 * returns true for every entry. It returns true if the predicate is satisfied for all entries,
 * and false otherwise.
 *
 * @template K - The type of keys in the Map.
 * @template V - The type of values in the Map.
 * @param {Map<K, V>} map - The Map to test.
 * @param {(value: V, key: K, map: Map<K, V>) => boolean} doesMatch - A predicate function that tests each entry.
 * @returns {boolean} true if all entries satisfy the predicate, false otherwise.
 *
 * @example
 * const map = new Map([
 *   ['a', 10],
 *   ['b', 20],
 *   ['c', 30]
 * ]);
 * const result = every(map, (value) => value > 5);
 * // result will be: true
 *
 * const result2 = every(map, (value) => value > 15);
 * // result2 will be: false
 */
declare function every<K, V>(map: Map<K, V>, doesMatch: (value: V, key: K, map: Map<K, V>) => boolean): boolean;

export { every };
