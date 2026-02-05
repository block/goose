/**
 * Reduces a Set to a single value by iterating through its elements and applying a callback function.
 *
 * This function iterates through all elements of the Set and applies the callback function to each element,
 * accumulating the result. If an initial value is provided, it is used as the starting accumulator value.
 * If no initial value is provided and the Set is empty, a TypeError is thrown.
 *
 * @template T - The type of elements in the Set.
 * @template A - The type of the accumulator.
 * @param {Set<T>} set - The Set to reduce.
 * @param {(accumulator: A, value: T, value2: T, set: Set<T>) => A} callback - A function that processes each element and updates the accumulator.
 * @param {A} [initialValue] - The initial value for the accumulator. If not provided, the first element in the Set is used.
 * @returns {A} The final accumulated value.
 * @throws {TypeError} If the Set is empty and no initial value is provided.
 *
 * @example
 * const set = new Set([1, 2, 3]);
 * const result = reduce(set, (acc, value) => acc + value, 0);
 * // result will be: 6
 *
 * @example
 * const set = new Set([10, 20]);
 * const result = reduce(set, (acc, value) => acc + value);
 * // result will be: 30 (starts with first value 10)
 */
declare function reduce<T, A = T>(set: Set<T>, callback: (accumulator: A, value: T, value2: T, set: Set<T>) => A, initialValue?: A): A;

export { reduce };
