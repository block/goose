/**
 * Counts the occurrences of items in a Set based on a transformation function.
 *
 * This function takes a Set and a function that generates a key from each value.
 * It returns a Map with the generated keys and their counts as values.
 * The count is incremented for each element for which the transformation produces the same key.
 *
 * @template T - The type of elements in the Set.
 * @template K - The type of keys produced by the transformation function.
 * @param {Set<T>} set - The Set to count occurrences from.
 * @param {(value: T, value2: T, set: Set<T>) => K} mapper - The function to produce a key for counting.
 * @returns {Map<K, number>} A Map containing the mapped keys and their counts.
 *
 * @example
 * const set = new Set([1, 2, 3, 4, 5]);
 * const result = countBy(set, (value) => value % 2 === 0 ? 'even' : 'odd');
 * // result will be Map(2) { 'odd' => 3, 'even' => 2 }
 *
 * @example
 * const set = new Set(['apple', 'banana', 'cherry']);
 * const result = countBy(set, (value) => value.length);
 * // result will be Map(2) { 5 => 1, 6 => 2 }
 */
declare function countBy<T, K>(set: Set<T>, mapper: (value: T, value2: T, set: Set<T>) => K): Map<K, number>;

export { countBy };
