/**
 * Creates a new function that always returns `value`.
 *
 * @template T - The type of the value to return.
 * @param {T} value - The value to return from the new function.
 * @returns {() => T} Returns the new constant function.
 */
declare function constant<T>(value: T): () => T;

export { constant };
