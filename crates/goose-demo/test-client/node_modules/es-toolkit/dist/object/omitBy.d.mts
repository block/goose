/**
 * Creates a new object composed of the properties that do not satisfy the predicate function.
 *
 * This function takes an object and a predicate function, and returns a new object that
 * includes only the properties for which the predicate function returns false.
 *
 * @template T - The type of object.
 * @param {T} obj - The object to omit properties from.
 * @param {(value: T[string], key: keyof T) => boolean} shouldOmit - A predicate function that determines
 * whether a property should be omitted. It takes the property's key and value as arguments and returns `true`
 * if the property should be omitted, and `false` otherwise.
 * @returns {Partial<T>} A new object with the properties that do not satisfy the predicate function.
 *
 * @example
 * const obj = { a: 1, b: 'omit', c: 3 };
 * const shouldOmit = (value) => typeof value === 'string';
 * const result = omitBy(obj, shouldOmit);
 * // result will be { a: 1, c: 3 }
 */
declare function omitBy<T extends Record<string, any>>(obj: T, shouldOmit: (value: T[keyof T], key: keyof T) => boolean): Partial<T>;

export { omitBy };
