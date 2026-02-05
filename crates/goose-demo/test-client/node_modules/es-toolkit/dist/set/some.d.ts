/**
 * Tests whether at least one element in a Set satisfies the provided predicate function.
 *
 * This function iterates through the elements of the Set and checks if the predicate function
 * returns true for at least one element. It returns true if any element satisfies the predicate,
 * and false otherwise.
 *
 * @template T - The type of elements in the Set.
 * @param {Set<T>} set - The Set to test.
 * @param {(value: T, value2: T, set: Set<T>) => boolean} doesMatch - A predicate function that tests each element.
 * @returns {boolean} true if at least one element satisfies the predicate, false otherwise.
 *
 * @example
 * const set = new Set([1, 2, 3]);
 * const result = some(set, (value) => value > 2);
 * // result will be: true
 *
 * const result2 = some(set, (value) => value > 5);
 * // result2 will be: false
 */
declare function some<T>(set: Set<T>, doesMatch: (value: T, value2: T, set: Set<T>) => boolean): boolean;

export { some };
