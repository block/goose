/**
 * Finds the element in an array that has the maximum value when applying
 * the `getValue` function to each element.
 *
 * @template T - The type of elements in the array.
 * @param {[T, ...T[]]} items The nonempty array of elements to search.
 * @param {(element: T) => number} getValue A function that selects a numeric value from each element.
 * @returns {T} The element with the maximum value as determined by the `getValue` function.
 * @example
 * maxBy([{ a: 1 }, { a: 2 }, { a: 3 }], x => x.a); // Returns: { a: 3 }
 * maxBy([], x => x.a); // Returns: undefined
 * maxBy(
 *   [
 *     { name: 'john', age: 30 },
 *     { name: 'jane', age: 28 },
 *     { name: 'joe', age: 26 },
 *   ],
 *   x => x.age
 * ); // Returns: { name: 'john', age: 30 }
 */
declare function maxBy<T>(items: readonly [T, ...T[]], getValue: (element: T, index: number, array: readonly T[]) => number): T;
/**
 * Finds the element in an array that has the maximum value when applying
 * the `getValue` function to each element.
 *
 * @template T - The type of elements in the array.
 * @param {T[]} items The array of elements to search.
 * @param {(element: T, index: number, array: readonly T[]) => number} getValue A function that selects a numeric value from each element.
 * @returns {T | undefined} The element with the maximum value as determined by the `getValue` function,
 * or `undefined` if the array is empty.
 * @example
 * maxBy([{ a: 1 }, { a: 2 }, { a: 3 }], x => x.a); // Returns: { a: 3 }
 * maxBy([], x => x.a); // Returns: undefined
 * maxBy(
 *   [
 *     { name: 'john', age: 30 },
 *     { name: 'jane', age: 28 },
 *     { name: 'joe', age: 26 },
 *   ],
 *   x => x.age
 * ); // Returns: { name: 'john', age: 30 }
 */
declare function maxBy<T>(items: readonly T[], getValue: (element: T, index: number, array: readonly T[]) => number): T | undefined;

export { maxBy };
