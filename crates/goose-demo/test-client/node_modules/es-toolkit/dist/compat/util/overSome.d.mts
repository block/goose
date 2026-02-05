/**
 * Creates a predicate function that checks if a value satisfies at least one of the given predicates.
 *
 * @template T - The type of the value to be checked.
 * @template U - The first possible type that the value could match.
 * @template V - The second possible type that the value could match.
 *
 * @param {(value: T) => value is U} predicate1 - A function that checks if the value matches type `U`.
 * @param {(value: T) => value is V} predicate2 - A function that checks if the value matches type `V`.
 *
 * @returns {(value: T) => value is U | V} A function that takes a value and returns `true` if any predicates return truthy.
 *
 * @example
 * const func = overSome(
 *   (value) => typeof value === 'string',
 *   (value) => typeof value === 'number'
 * );
 *
 * func("hello"); // true
 * func(42); // true
 * func([]); // false
 */
declare function overSome<T, U extends T, V extends T>(predicate1: (value: T) => value is U, predicate2: (value: T) => value is V): (value: T) => value is U | V;
/**
 * Creates a function that checks if any of the given predicates return truthy for the provided values.
 *
 * @template T - The type of the values to be checked.
 *
 * @param {...Array<((...values: T[]) => boolean) | ReadonlyArray<(...values: T[]) => boolean>>} predicates -
 *   A list of predicates or arrays of predicates. Each predicate is a function that takes one or more values of
 *   type `T` and returns a boolean indicating whether the condition is satisfied for those values.
 *
 * @returns {(...values: T[]) => boolean} A function that takes a list of values and returns `true` if any of the
 *   predicates return truthy for the provided values, and `false` otherwise.
 *
 * @example
 * const func = overSome(
 *   (value) => typeof value === 'string',
 *   (value) => typeof value === 'number',
 *   (value) => typeof value === 'symbol'
 * );
 *
 * func("hello"); // true
 * func(42); // true
 * func(Symbol()); // true
 * func([]); // false
 *
 * @example
 * const func = overSome([
 *   (value) => value.a > 0,
 *   (value) => value.b > 0
 * ]);
 *
 * func({ a: 0, b: 2 }); // true
 * func({ a: 0, b: 0 }); // false
 *
 * @example
 * const func = overSome(
 *   (a, b) => typeof a === 'string' && typeof b === 'string',
 *   (a, b) => a > 0 && b > 0
 * );
 *
 * func("hello", "world"); // true
 * func(1, 10); // true
 * func(0, 2); // false
 */
declare function overSome<T>(...predicates: Array<((...values: T[]) => boolean) | ReadonlyArray<(...values: T[]) => boolean>>): (...values: T[]) => boolean;

export { overSome };
