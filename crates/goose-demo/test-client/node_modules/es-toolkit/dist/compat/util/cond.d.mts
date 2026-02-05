/**
 * Creates a function that checks conditions one by one and runs the matching function.
 *
 * Each pair consists of a condition (predicate) and a function to run.
 * The function goes through each condition in order until it finds one that's true.
 * When it finds a true condition, it runs the corresponding function and returns its result.
 * If none of the conditions are true, it returns undefined.
 *
 * @param {Array<Array>} pairs - Array of pairs. Each pair consists of a predicate function and a function to run.
 * @returns {(...args: any[]) => unknown} A new composite function that checks conditions and runs the matching function.
 * @example
 *
 * const func = cond([
 *   [matches({ a: 1 }), constant('matches A')],
 *   [conforms({ b: isNumber }), constant('matches B')],
 *   [stubTrue, constant('no match')]
 * ]);
 *
 * func({ a: 1, b: 2 });
 * // => 'matches A'
 *
 * func({ a: 0, b: 1 });
 * // => 'matches B'
 *
 * func({ a: '1', b: '2' });
 * // => 'no match'
 */
declare function cond<R>(pairs: Array<[truthy: () => boolean, falsey: () => R]>): () => R;
/**
 * Creates a function that checks conditions one by one and runs the matching function.
 *
 * Each pair consists of a condition (predicate) and a function to run.
 * The function goes through each condition in order until it finds one that's true.
 * When it finds a true condition, it runs the corresponding function and returns its result.
 * If none of the conditions are true, it returns undefined.
 *
 * @param {Array<Array>} pairs - Array of pairs. Each pair consists of a predicate function and a function to run.
 * @returns {(...args: any[]) => unknown} A new composite function that checks conditions and runs the matching function.
 * @example
 *
 * const func = cond([
 *   [matches({ a: 1 }), constant('matches A')],
 *   [conforms({ b: isNumber }), constant('matches B')],
 *   [stubTrue, constant('no match')]
 * ]);
 *
 * func({ a: 1, b: 2 });
 * // => 'matches A'
 *
 * func({ a: 0, b: 1 });
 * // => 'matches B'
 *
 * func({ a: '1', b: '2' });
 * // => 'no match'
 */
declare function cond<T, R>(pairs: Array<[truthy: (val: T) => boolean, falsey: (val: T) => R]>): (val: T) => R;

export { cond };
