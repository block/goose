/**
 * Creates a new Map with the same keys but with values transformed by the provided function.
 *
 * This function takes a Map and a function that generates a new value from each value-key pair.
 * It returns a new Map where the values are the result of applying the function to each entry,
 * while the keys remain the same.
 *
 * @template K - The type of keys in the Map.
 * @template V - The type of values in the Map.
 * @param {Map<K, V>} map - The Map to transform.
 * @param {(value: V, key: K, object: Map<K, V>) => V} getNewValue - A function that generates a new value from a value-key pair.
 * @returns {Map<K, V>} A new Map with the same keys and transformed values.
 *
 * @example
 * const map = new Map([
 *   ['a', 1],
 *   ['b', 2],
 *   ['c', 3]
 * ]);
 * const result = mapValues(map, (value) => value * 2);
 * // result will be:
 * // Map(3) {
 * //   'a' => 2,
 * //   'b' => 4,
 * //   'c' => 6
 * // }
 */
declare function mapValues<K, V, R>(map: Map<K, V>, getNewValue: (value: V, key: K, object: Map<K, V>) => R): Map<K, R>;

export { mapValues };
