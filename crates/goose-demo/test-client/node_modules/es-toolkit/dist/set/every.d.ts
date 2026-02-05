/**
 * Tests whether all elements in a Set satisfy the provided predicate function.
 *
 * This function iterates through all elements of the Set and checks if the predicate function
 * returns true for every element. It returns true if the predicate is satisfied for all elements,
 * and false otherwise.
 *
 * @template T - The type of elements in the Set.
 * @param {Set<T>} set - The Set to test.
 * @param {(value: T, value2: T, set: Set<T>) => boolean} doesMatch - A predicate function that tests each element.
 * @returns {boolean} true if all elements satisfy the predicate, false otherwise.
 *
 * @example
 * const set = new Set([10, 20, 30]);
 * const result = every(set, (value) => value > 5);
 * // result will be: true
 *
 * const result2 = every(set, (value) => value > 15);
 * // result2 will be: false
 */
declare function every<T>(set: Set<T>, doesMatch: (value: T, value2: T, set: Set<T>) => boolean): boolean;

export { every };
