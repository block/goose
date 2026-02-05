/**
 * Creates a predicate function that checks if a value satisfies all of the given predicates.
 *
 * @template T - The type of the value to be checked.
 * @template U - The first possible type that the value could match.
 * @template V - The second possible type that the value could match.
 *
 * @param {(value: T) => value is U} predicate1 - A function that checks if the value matches type `U`.
 * @param {(value: T) => value is V} predicate2 - A function that checks if the value matches type `V`.
 *
 * @returns {(value: T) => value is U & V} A function that takes a value and returns `true` if all predicates return truthy.
 *
 * @example
 * const func = overEvery(
 *   (value) => typeof value === 'string',
 *   (value) => value === 'hello'
 * );
 *
 * func("hello"); // true
 * func("world"); // false
 * func(42); // false
 */
declare function overEvery<T, U extends T, V extends T>(predicate1: (value: T) => value is U, predicate2: (value: T) => value is V): (value: T) => value is U & V;
/**
 * Creates a function that checks if all of the given predicates return truthy for the provided values.
 *
 * @template T - The type of the values to be checked.
 *
 * @param {...Array<((...values: T[]) => boolean) | ReadonlyArray<(...values: T[]) => boolean>>} predicates -
 *   A list of predicates or arrays of predicates. Each predicate is a function that takes one or more values of
 *   type `T` and returns a boolean indicating whether the condition is satisfied for those values.
 *
 * @returns {(...values: T[]) => boolean} A function that takes a list of values and returns `true` if all of the
 *   predicates return truthy for the provided values, and `false` otherwise.
 *
 * @example
 * const func = overEvery(
 *   (value) => typeof value === 'string',
 *   (value) => value.length > 3
 * );
 *
 * func("hello"); // true
 * func("hi"); // false
 * func(42); // false
 *
 * @example
 * const func = overEvery([
 *   (value) => value.a > 0,
 *   (value) => value.b > 0
 * ]);
 *
 * func({ a: 1, b: 2 }); // true
 * func({ a: 0, b: 2 }); // false
 *
 * @example
 * const func = overEvery(
 *   (a, b) => typeof a === 'string' && typeof b === 'string',
 *   (a, b) => a.length > 3 && b.length > 3
 * );
 *
 * func("hello", "world"); // true
 * func("hi", "world"); // false
 * func(1, 10); // false
 */
declare function overEvery<T>(...predicates: Array<((...args: T[]) => boolean) | ReadonlyArray<(...args: T[]) => boolean>>): (...args: T[]) => boolean;

export { overEvery };
