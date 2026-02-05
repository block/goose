type CloneDeepWithCustomizer<TObject> = (value: any, key: number | string | undefined, object: TObject | undefined, stack: any) => any;
/**
 * Creates a deep clone of the given value using a customizer function.
 *
 * @template T - The type of the value.
 * @param {T} value - The value to clone.
 * @param {CloneDeepWithCustomizer<T>} customizer - A function to customize the cloning process.
 * @returns {any} - A deep clone of the given value.
 *
 * @example
 * const obj = { a: 1, b: 2 };
 * const clonedObj = cloneDeepWith(obj, (value) => {
 *   if (typeof value === 'number') {
 *     return value * 2;
 *   }
 * });
 * // => { a: 2, b: 4 }
 */
declare function cloneDeepWith<T>(value: T, customizer: CloneDeepWithCustomizer<T>): any;
/**
 * Creates a deep clone of the given value.
 *
 * @template T - The type of the value.
 * @param {T} value - The value to clone.
 * @returns {T} - A deep clone of the given value.
 *
 * @example
 * const obj = { a: 1, b: { c: 2 } };
 * const clonedObj = cloneDeepWith(obj);
 * // => { a: 1, b: { c: 2 } }
 */
declare function cloneDeepWith<T>(value: T): T;

export { cloneDeepWith };
