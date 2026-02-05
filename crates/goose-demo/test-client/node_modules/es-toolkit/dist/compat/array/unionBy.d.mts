import { ValueIteratee } from '../_internal/ValueIteratee.mjs';

/**
 * This method is like `union` except that it accepts `iteratee` which is
 * invoked for each element of each `arrays` to generate the criterion by which
 * uniqueness is computed. The iteratee is invoked with one argument: (value).
 *
 * @template T
 * @param {ArrayLike<T> | null | undefined} arrays - The arrays to inspect.
 * @param {ValueIteratee<T>} [iteratee] - The iteratee invoked per element.
 * @returns {T[]} Returns the new array of combined values.
 *
 * @example
 * unionBy([2.1], [1.2, 2.3], Math.floor);
 * // => [2.1, 1.2]
 *
 * @example
 * unionBy([{ 'x': 1 }], [{ 'x': 2 }, { 'x': 1 }], 'x');
 * // => [{ 'x': 1 }, { 'x': 2 }]
 */
declare function unionBy<T>(arrays: ArrayLike<T> | null | undefined, iteratee?: ValueIteratee<T>): T[];
/**
 * This method is like `union` except that it accepts `iteratee` which is
 * invoked for each element of each `arrays` to generate the criterion by which
 * uniqueness is computed. The iteratee is invoked with one argument: (value).
 *
 * @template T
 * @param {ArrayLike<T> | null | undefined} arrays1 - The first array to inspect.
 * @param {ArrayLike<T> | null | undefined} arrays2 - The second array to inspect.
 * @param {ValueIteratee<T>} [iteratee] - The iteratee invoked per element.
 * @returns {T[]} Returns the new array of combined values.
 *
 * @example
 * unionBy([2.1], [1.2, 2.3], Math.floor);
 * // => [2.1, 1.2]
 */
declare function unionBy<T>(arrays1: ArrayLike<T> | null | undefined, arrays2: ArrayLike<T> | null | undefined, iteratee?: ValueIteratee<T>): T[];
/**
 * This method is like `union` except that it accepts `iteratee` which is
 * invoked for each element of each `arrays` to generate the criterion by which
 * uniqueness is computed. The iteratee is invoked with one argument: (value).
 *
 * @template T
 * @param {ArrayLike<T> | null | undefined} arrays1 - The first array to inspect.
 * @param {ArrayLike<T> | null | undefined} arrays2 - The second array to inspect.
 * @param {ArrayLike<T> | null | undefined} arrays3 - The third array to inspect.
 * @param {ValueIteratee<T>} [iteratee] - The iteratee invoked per element.
 * @returns {T[]} Returns the new array of combined values.
 *
 * @example
 * unionBy([2.1], [1.2, 2.3], [3.4], Math.floor);
 * // => [2.1, 1.2, 3.4]
 */
declare function unionBy<T>(arrays1: ArrayLike<T> | null | undefined, arrays2: ArrayLike<T> | null | undefined, arrays3: ArrayLike<T> | null | undefined, iteratee?: ValueIteratee<T>): T[];
/**
 * This method is like `union` except that it accepts `iteratee` which is
 * invoked for each element of each `arrays` to generate the criterion by which
 * uniqueness is computed. The iteratee is invoked with one argument: (value).
 *
 * @template T
 * @param {ArrayLike<T> | null | undefined} arrays1 - The first array to inspect.
 * @param {ArrayLike<T> | null | undefined} arrays2 - The second array to inspect.
 * @param {ArrayLike<T> | null | undefined} arrays3 - The third array to inspect.
 * @param {ArrayLike<T> | null | undefined} arrays4 - The fourth array to inspect.
 * @param {ValueIteratee<T>} [iteratee] - The iteratee invoked per element.
 * @returns {T[]} Returns the new array of combined values.
 *
 * @example
 * unionBy([2.1], [1.2, 2.3], [3.4], [4.5], Math.floor);
 * // => [2.1, 1.2, 3.4, 4.5]
 */
declare function unionBy<T>(arrays1: ArrayLike<T> | null | undefined, arrays2: ArrayLike<T> | null | undefined, arrays3: ArrayLike<T> | null | undefined, arrays4: ArrayLike<T> | null | undefined, iteratee?: ValueIteratee<T>): T[];
/**
 * This method is like `union` except that it accepts `iteratee` which is
 * invoked for each element of each `arrays` to generate the criterion by which
 * uniqueness is computed. The iteratee is invoked with one argument: (value).
 *
 * @template T
 * @param {ArrayLike<T> | null | undefined} arrays1 - The first array to inspect.
 * @param {ArrayLike<T> | null | undefined} arrays2 - The second array to inspect.
 * @param {ArrayLike<T> | null | undefined} arrays3 - The third array to inspect.
 * @param {ArrayLike<T> | null | undefined} arrays4 - The fourth array to inspect.
 * @param {ArrayLike<T> | null | undefined} arrays5 - The fifth array to inspect.
 * @param {...Array<ValueIteratee<T> | ArrayLike<T> | null | undefined>} iteratee - The iteratee invoked per element.
 * @returns {T[]} Returns the new array of combined values.
 *
 * @example
 * unionBy([2.1], [1.2, 2.3], [3.4], [4.5], [5.6], Math.floor);
 * // => [2.1, 1.2, 3.4, 4.5, 5.6]
 */
declare function unionBy<T>(arrays1: ArrayLike<T> | null | undefined, arrays2: ArrayLike<T> | null | undefined, arrays3: ArrayLike<T> | null | undefined, arrays4: ArrayLike<T> | null | undefined, arrays5: ArrayLike<T> | null | undefined, ...iteratee: Array<ValueIteratee<T> | ArrayLike<T> | null | undefined>): T[];

export { unionBy };
