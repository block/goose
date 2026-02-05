import { ObjectIteratee } from '../_internal/ObjectIteratee.js';

/**
 * Finds the key of the last element that matches the given predicate.
 *
 * This function determines the type of the predicate and delegates the search
 * to the appropriate helper function. It supports predicates as functions, objects,
 * arrays, or strings.
 *
 * @template T - The type of the object.
 * @param {T | null | undefined} obj - The object to inspect.
 * @param {ObjectIteratee<T>} predicate - The predicate to match.
 * @returns {string | undefined} Returns the key of the matched element, else `undefined`.
 */
declare function findLastKey<T>(obj: T | null | undefined, predicate?: ObjectIteratee<T>): string | undefined;

export { findLastKey };
