/**
 * Merges the properties of the source object into the target object.
 *
 * You can provide a custom `merge` function to control how properties are merged. It should return the value to be set in the target object.
 *
 * If it returns `undefined`, a default deep merge will be applied for arrays and objects:
 *
 * - If a property in the source object is an array or an object and the corresponding property in the target object is also an array or object, they will be merged.
 * - If a property in the source object is undefined, it will not overwrite a defined property in the target object.
 *
 * Note that this function mutates the target object.
 *
 * @param {T} target - The target object into which the source object properties will be merged. This object is modified in place.
 * @param {S} source - The source object whose properties will be merged into the target object.
 * @param {(targetValue: any, sourceValue: any, key: string, target: T, source: S) => any} merge - A custom merge function that defines how properties should be combined. It receives the following arguments:
 *   - `targetValue`: The current value of the property in the target object.
 *   - `sourceValue`: The value of the property in the source object.
 *   - `key`: The key of the property being merged.
 *   - `target`: The target object.
 *   - `source`: The source object.
 *
 * @returns {T & S} The updated target object with properties from the source object merged in.
 *
 * @template T - Type of the target object.
 * @template S - Type of the source object.
 *
 * @example
 * const target = { a: 1, b: 2 };
 * const source = { b: 3, c: 4 };
 *
 * mergeWith(target, source, (targetValue, sourceValue) => {
 *   if (typeof targetValue === 'number' && typeof sourceValue === 'number') {
 *     return targetValue + sourceValue;
 *   }
 * });
 * // Returns { a: 1, b: 5, c: 4 }
 * @example
 * const target = { a: [1], b: [2] };
 * const source = { a: [3], b: [4] };
 *
 * const result = mergeWith(target, source, (objValue, srcValue) => {
 *   if (Array.isArray(objValue)) {
 *     return objValue.concat(srcValue);
 *   }
 * });
 *
 * expect(result).toEqual({ a: [1, 3], b: [2, 4] });
 */
declare function mergeWith<T extends Record<PropertyKey, any>, S extends Record<PropertyKey, any>>(target: T, source: S, merge: (targetValue: any, sourceValue: any, key: string, target: T, source: S) => any): T & S;

export { mergeWith };
