import { ValueIteratee } from '../_internal/ValueIteratee.mjs';

/**
 * Finds the element in an array that has the minimum value when applying
 * the `iteratee` to each element.
 *
 * @template T - The type of elements in the array.
 * @param {T[]} items The array of elements to search.
 * @param {((element: T) => number) | keyof T | [keyof T, unknown] | Partial<T>} iteratee
 * The criteria used to determine the minimum value.
 *  - If a **function** is provided, it extracts a numeric value from each element.
 *  - If a **string** is provided, it is treated as a key to extract values from the objects.
 *  - If a **[key, value]** pair is provided, it matches elements with the specified key-value pair.
 *  - If an **object** is provided, it matches elements that contain the specified properties.
 * @returns {T | undefined} The element with the minimum value as determined by the `iteratee`.
 * @example
 * minBy([{ a: 1 }, { a: 2 }, { a: 3 }], x => x.a); // Returns: { a: 1 }
 * minBy([], x => x.a); // Returns: undefined
 * minBy(
 *   [
 *     { name: 'john', age: 30 },
 *     { name: 'jane', age: 28 },
 *     { name: 'joe', age: 26 },
 *   ],
 *   x => x.age
 * ); // Returns: { name: 'joe', age: 26 }
 * minBy([{ a: 1 }, { a: 2 }], 'a'); // Returns: { a: 1 }
 * minBy([{ a: 1 }, { a: 2 }], ['a', 1]); // Returns: { a: 2 }
 * minBy([{ a: 1 }, { a: 2 }], { a: 1 }); // Returns: { a: 2 }
 */
declare function minBy<T>(items: ArrayLike<T> | null | undefined, iteratee?: ValueIteratee<T>): T | undefined;

export { minBy };
