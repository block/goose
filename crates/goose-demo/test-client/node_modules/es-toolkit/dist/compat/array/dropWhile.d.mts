import { ListIteratee } from '../_internal/ListIteratee.mjs';

/**
 * Creates a slice of array excluding elements dropped from the beginning.
 * Elements are dropped until predicate returns falsey.
 * The predicate is invoked with three arguments: (value, index, array).
 *
 * @template T - The type of elements in the array
 * @param {ArrayLike<T> | null | undefined} arr - The array to query
 * @param {ListIteratee<T>} [predicate=identity] - The function invoked per iteration
 * @returns {T[]} Returns the slice of array
 *
 * @example
 * dropWhile([1, 2, 3], n => n < 3)
 * // => [3]
 *
 * dropWhile([{ a: 1, b: 2 }, { a: 1, b: 3 }], { a: 1 })
 * // => [{ a: 1, b: 3 }]
 *
 * dropWhile([{ a: 1, b: 2 }, { a: 1, b: 3 }], ['a', 1])
 * // => [{ a: 1, b: 3 }]
 *
 * dropWhile([{ a: 1, b: 2 }, { a: 1, b: 3 }], 'a')
 * // => []
 */
declare function dropWhile<T>(arr: ArrayLike<T> | null | undefined, predicate?: ListIteratee<T>): T[];

export { dropWhile };
