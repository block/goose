import { ValueIteratee } from '../_internal/ValueIteratee.js';

/**
 * Finds the element in an array that has the maximum value when applying
 * the `iteratee` to each element.
 *
 * @template T - The type of elements in the array.
 * @param {ArrayLike<T> | null | undefined} items The array of elements to search.
 * @param {ValueIteratee<T>} iteratee
 * The criteria used to determine the maximum value.
 *  - If a **function** is provided, it extracts a numeric value from each element.
 *  - If a **string** is provided, it is treated as a key to extract values from the objects.
 *  - If a **[key, value]** pair is provided, it matches elements with the specified key-value pair.
 *  - If an **object** is provided, it matches elements that contain the specified properties.
 * @returns {T | undefined} The element with the maximum value as determined by the `iteratee`.
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
 * maxBy([{ a: 1 }, { a: 2 }], 'a'); // Returns: { a: 2 }
 * maxBy([{ a: 1 }, { a: 2 }], ['a', 1]); // Returns: { a: 1 }
 * maxBy([{ a: 1 }, { a: 2 }], { a: 1 }); // Returns: { a: 1 }
 */
declare function maxBy<T>(items: ArrayLike<T> | null | undefined, iteratee?: ValueIteratee<T>): T | undefined;

export { maxBy };
