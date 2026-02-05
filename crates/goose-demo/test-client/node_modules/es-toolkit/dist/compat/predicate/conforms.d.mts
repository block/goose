import { ConformsPredicateObject } from '../_internal/ConformsPredicateObject.mjs';

/**
 * Creates a function that invokes the predicate properties of `source` with the corresponding property values of a given object, returning `true` if all predicates return truthy, else `false`.
 *
 * Note: The created function is equivalent to `conformsTo` with source partially applied.
 *
 * @param {Record<PropertyKey, (value: any) => boolean>} source The object of property predicates to conform to.
 * @returns {(object: Record<PropertyKey, any>) => boolean} Returns the new spec function.
 *
 * @example
 * const isPositive = (n) => n > 0;
 * const isEven = (n) => n % 2 === 0;
 * const predicates = { a: isPositive, b: isEven };
 * const conform = conforms(predicates);
 *
 * console.log(conform({ a: 2, b: 4 })); // true
 * console.log(conform({ a: -1, b: 4 })); // false
 * console.log(conform({ a: 2, b: 3 })); // false
 * console.log(conform({ a: 0, b: 2 })); // false
 */
declare function conforms<T>(source: ConformsPredicateObject<T>): (value: T) => boolean;

export { conforms };
