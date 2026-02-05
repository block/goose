/**
 * Reduces a Map to a single value by iterating through its entries and applying a callback function.
 *
 * This function iterates through all entries of the Map and applies the callback function to each entry,
 * accumulating the result. If an initial value is provided, it is used as the starting accumulator value.
 * If no initial value is provided and the Map is empty, a TypeError is thrown.
 *
 * @template K - The type of keys in the Map.
 * @template V - The type of values in the Map.
 * @param {Map<K, V>} map - The Map to reduce.
 * @param {(accumulator: V, value: V, key: K, map: Map<K, V>) => V} callback - A function that processes each entry and updates the accumulator.
 * @param {V} [initialValue] - The initial value for the accumulator. If not provided, the first value in the Map is used.
 * @returns {V} The final accumulated value.
 * @throws {TypeError} If the Map is empty and no initial value is provided.
 *
 * @example
 * const map = new Map([
 *   ['a', 1],
 *   ['b', 2],
 *   ['c', 3]
 * ]);
 * const result = reduce(map, (acc, value) => acc + value, 0);
 * // result will be: 6
 *
 * @example
 * const map = new Map([
 *   ['a', 10],
 *   ['b', 20]
 * ]);
 * const result = reduce(map, (acc, value) => acc + value);
 * // result will be: 30 (starts with first value 10)
 */
declare function reduce<K, V, A = V>(map: Map<K, V>, callback: (accumulator: A, value: V, key: K, map: Map<K, V>) => A, initialValue?: A): A;

export { reduce };
