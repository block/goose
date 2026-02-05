/**
 * Creates a new Set with elements transformed by the provided function.
 *
 * This function takes a Set and a function that generates a new value from each element.
 * It returns a new Set where the elements are the result of applying the function to each element.
 *
 * @template T - The type of elements in the input Set.
 * @template U - The type of elements in the output Set.
 * @param {Set<T>} set - The Set to transform.
 * @param {(value: T, value2: T, set: Set<T>) => U} getNewValue - A function that generates a new value from an element.
 * @returns {Set<U>} A new Set with transformed elements.
 *
 * @example
 * const set = new Set([1, 2, 3]);
 * const result = map(set, (value) => value * 2);
 * // result will be:
 * // Set(3) { 2, 4, 6 }
 */
declare function map<T, U>(set: Set<T>, getNewValue: (value: T, value2: T, set: Set<T>) => U): Set<U>;

export { map };
