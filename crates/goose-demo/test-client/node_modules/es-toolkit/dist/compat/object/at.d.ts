type PropertyName = string | number | symbol;
type Many<T> = T | readonly T[];
type PropertyPath = Many<PropertyName>;
/**
 * Gets values at given paths from a dictionary or numeric dictionary.
 *
 * @template T - The type of the values in the dictionary.
 * @param {Record<string, T> | Record<number, T> | null | undefined} object - The dictionary to query.
 * @param {...PropertyPath[]} props - The property paths to get values for.
 * @returns {T[]} Returns an array of the picked values.
 *
 * @example
 * const object = { 'a': 1, 'b': 2, 'c': 3 };
 * at(object, 'a', 'c');
 * // => [1, 3]
 */
declare function at<T>(object: Record<string, T> | Record<number, T> | null | undefined, ...props: PropertyPath[]): T[];
/**
 * Gets values at given keys from an object.
 *
 * @template T - The type of the object.
 * @param {T | null | undefined} object - The object to query.
 * @param {...Array<Many<keyof T>>} props - The property keys to get values for.
 * @returns {Array<T[keyof T]>} Returns an array of the picked values.
 *
 * @example
 * const object = { 'a': 1, 'b': 2, 'c': 3 };
 * at(object, 'a', 'c');
 * // => [1, 3]
 */
declare function at<T extends object>(object: T | null | undefined, ...props: Array<Many<keyof T>>): Array<T[keyof T]>;

export { at };
