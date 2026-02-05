import { ValueIteratee } from '../_internal/ValueIteratee.js';

/**
 * Creates an object composed of keys generated from the results of running each element of collection through
 * iteratee. The corresponding value of each key is the number of times the key was returned by iteratee. The
 * iteratee is invoked with one argument: (value).
 *
 * @param collection The collection to iterate over.
 * @param iteratee The function invoked per iteration.
 * @return Returns the composed aggregate object.
 *
 * @example
 * countBy([6.1, 4.2, 6.3], Math.floor); // => { '4': 1, '6': 2 }
 * countBy(['one', 'two', 'three'], 'length'); // => { '3': 2, '5': 1 }
 */
declare function countBy<T>(collection: ArrayLike<T> | null | undefined, iteratee?: ValueIteratee<T>): Record<string, number>;
declare function countBy<T extends object>(collection: T | null | undefined, iteratee?: ValueIteratee<T[keyof T]>): Record<string, number>;

export { countBy };
