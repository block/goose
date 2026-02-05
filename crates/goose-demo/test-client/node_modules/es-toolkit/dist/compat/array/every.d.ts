import { ListIterateeCustom } from '../_internal/ListIterateeCustom.js';
import { ObjectIterateeCustom } from '../_internal/ObjectIteratee.js';

/**
 * Checks if all elements in a collection pass the predicate check.
 * The predicate is invoked with three arguments: (value, index|key, collection).
 *
 * @template T - The type of elements in the collection
 * @param {ArrayLike<T> | null | undefined} collection - The collection to iterate over
 * @param {ListIterateeCustom<T, boolean>} [predicate=identity] - The function invoked per iteration
 * @returns {boolean} Returns true if all elements pass the predicate check, else false
 *
 * @example
 * // Using a function predicate
 * every([true, 1, null, 'yes'], Boolean)
 * // => false
 *
 * // Using property shorthand
 * const users = [{ user: 'barney', age: 36 }, { user: 'fred', age: 40 }]
 * every(users, 'age')
 * // => true
 *
 * // Using matches shorthand
 * every(users, { age: 36 })
 * // => false
 *
 * // Using matchesProperty shorthand
 * every(users, ['age', 36])
 * // => false
 */
declare function every<T>(collection: ArrayLike<T> | null | undefined, predicate?: ListIterateeCustom<T, boolean>): boolean;
/**
 * Checks if all elements in an object pass the predicate check.
 * The predicate is invoked with three arguments: (value, key, object).
 *
 * @template T - The type of the object
 * @param {T | null | undefined} collection - The object to iterate over
 * @param {ObjectIterateeCustom<T, boolean>} [predicate=identity] - The function invoked per iteration
 * @returns {boolean} Returns true if all elements pass the predicate check, else false
 *
 * @example
 * // Using a function predicate
 * every({ a: true, b: 1, c: null }, Boolean)
 * // => false
 *
 * // Using property shorthand
 * const users = {
 *   barney: { active: true, age: 36 },
 *   fred: { active: true, age: 40 }
 * }
 * every(users, 'active')
 * // => true
 *
 * // Using matches shorthand
 * every(users, { active: true })
 * // => true
 *
 * // Using matchesProperty shorthand
 * every(users, ['age', 36])
 * // => false
 */
declare function every<T extends object>(collection: T | null | undefined, predicate?: ObjectIterateeCustom<T, boolean>): boolean;

export { every };
