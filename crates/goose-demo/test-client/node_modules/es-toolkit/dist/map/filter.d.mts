/**
 * Filters a Map based on a predicate function.
 *
 * This function takes a Map and a predicate function, and returns a new Map containing
 * only the entries for which the predicate function returns true.
 *
 * @template K - The type of keys in the Map.
 * @template V - The type of values in the Map.
 * @param {Map<K, V>} map - The Map to filter.
 * @param {(value: V, key: K, map: Map<K, V>) => boolean} callback - A predicate function that tests each entry.
 * @returns {Map<K, V>} A new Map containing only the entries that satisfy the predicate.
 *
 * @example
 * const map = new Map([
 *   ['a', 1],
 *   ['b', 2],
 *   ['c', 3],
 *   ['d', 4]
 * ]);
 * const result = filter(map, (value) => value > 2);
 * // result will be:
 * // Map(2) {
 * //   'c' => 3,
 * //   'd' => 4
 * // }
 */
declare function filter<K, V>(map: Map<K, V>, callback: (value: V, key: K, map: Map<K, V>) => boolean): Map<K, V>;

export { filter };
