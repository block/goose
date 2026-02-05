interface FlattenObjectOptions {
    /**
     * The delimiter to use between nested keys.
     * @default '.'
     */
    delimiter?: string;
}
/**
 * Flattens a nested object into a single level object with delimiter-separated keys.
 *
 * @param {object} object - The object to flatten.
 * @param {string} [options.delimiter='.'] - The delimiter to use between nested keys.
 * @returns {Record<string, any>} - The flattened object.
 *
 * @example
 * const nestedObject = {
 *   a: {
 *     b: {
 *       c: 1
 *     }
 *   },
 *   d: [2, 3]
 * };
 *
 * const flattened = flattenObject(nestedObject);
 * console.log(flattened);
 * // Output:
 * // {
 * //   'a.b.c': 1,
 * //   'd.0': 2,
 * //   'd.1': 3
 * // }
 */
declare function flattenObject(object: object, { delimiter }?: FlattenObjectOptions): Record<string, any>;

export { flattenObject };
