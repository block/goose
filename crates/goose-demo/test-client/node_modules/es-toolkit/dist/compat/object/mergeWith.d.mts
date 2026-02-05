type MergeWithCustomizer = (objValue: any, srcValue: any, key: string, object: any, source: any, stack: any) => any;
/**
 * Merges the properties of a source object into the target object using a customizer function.
 *
 * @template TObject
 * @template TSource
 * @param {TObject} object - The target object into which the source object properties will be merged.
 * @param {TSource} source - The source object whose properties will be merged into the target object.
 * @param {MergeWithCustomizer} customizer - The function to customize assigned values.
 * @returns {TObject & TSource} Returns the updated target object with properties from the source object merged in.
 *
 * @example
 * const target = { a: 1, b: 2 };
 * const source = { b: 3, c: 4 };
 *
 * const result = mergeWith(target, source, (objValue, srcValue) => {
 *   if (typeof objValue === 'number' && typeof srcValue === 'number') {
 *     return objValue + srcValue;
 *   }
 * });
 * // => { a: 1, b: 5, c: 4 }
 */
declare function mergeWith<TObject, TSource>(object: TObject, source: TSource, customizer: MergeWithCustomizer): TObject & TSource;
/**
 * Merges the properties of two source objects into the target object using a customizer function.
 *
 * @template TObject
 * @template TSource1
 * @template TSource2
 * @param {TObject} object - The target object into which the source objects properties will be merged.
 * @param {TSource1} source1 - The first source object.
 * @param {TSource2} source2 - The second source object.
 * @param {MergeWithCustomizer} customizer - The function to customize assigned values.
 * @returns {TObject & TSource1 & TSource2} Returns the updated target object with properties from the source objects merged in.
 *
 * @example
 * const target = { a: 1 };
 * const source1 = { b: 2 };
 * const source2 = { c: 3 };
 *
 * const result = mergeWith(target, source1, source2, (objValue, srcValue) => {
 *   if (typeof objValue === 'number' && typeof srcValue === 'number') {
 *     return objValue + srcValue;
 *   }
 * });
 * // => { a: 1, b: 2, c: 3 }
 */
declare function mergeWith<TObject, TSource1, TSource2>(object: TObject, source1: TSource1, source2: TSource2, customizer: MergeWithCustomizer): TObject & TSource1 & TSource2;
/**
 * Merges the properties of three source objects into the target object using a customizer function.
 *
 * @template TObject
 * @template TSource1
 * @template TSource2
 * @template TSource3
 * @param {TObject} object - The target object into which the source objects properties will be merged.
 * @param {TSource1} source1 - The first source object.
 * @param {TSource2} source2 - The second source object.
 * @param {TSource3} source3 - The third source object.
 * @param {MergeWithCustomizer} customizer - The function to customize assigned values.
 * @returns {TObject & TSource1 & TSource2 & TSource3} Returns the updated target object with properties from the source objects merged in.
 *
 * @example
 * const target = { a: 1 };
 * const source1 = { b: 2 };
 * const source2 = { c: 3 };
 * const source3 = { d: 4 };
 *
 * const result = mergeWith(target, source1, source2, source3, (objValue, srcValue) => {
 *   if (typeof objValue === 'number' && typeof srcValue === 'number') {
 *     return objValue + srcValue;
 *   }
 * });
 * // => { a: 1, b: 2, c: 3, d: 4 }
 */
declare function mergeWith<TObject, TSource1, TSource2, TSource3>(object: TObject, source1: TSource1, source2: TSource2, source3: TSource3, customizer: MergeWithCustomizer): TObject & TSource1 & TSource2 & TSource3;
/**
 * Merges the properties of four source objects into the target object using a customizer function.
 *
 * @template TObject
 * @template TSource1
 * @template TSource2
 * @template TSource3
 * @template TSource4
 * @param {TObject} object - The target object into which the source objects properties will be merged.
 * @param {TSource1} source1 - The first source object.
 * @param {TSource2} source2 - The second source object.
 * @param {TSource3} source3 - The third source object.
 * @param {TSource4} source4 - The fourth source object.
 * @param {MergeWithCustomizer} customizer - The function to customize assigned values.
 * @returns {TObject & TSource1 & TSource2 & TSource3 & TSource4} Returns the updated target object with properties from the source objects merged in.
 *
 * @example
 * const target = { a: 1 };
 * const source1 = { b: 2 };
 * const source2 = { c: 3 };
 * const source3 = { d: 4 };
 * const source4 = { e: 5 };
 *
 * const result = mergeWith(target, source1, source2, source3, source4, (objValue, srcValue) => {
 *   if (typeof objValue === 'number' && typeof srcValue === 'number') {
 *     return objValue + srcValue;
 *   }
 * });
 * // => { a: 1, b: 2, c: 3, d: 4, e: 5 }
 */
declare function mergeWith<TObject, TSource1, TSource2, TSource3, TSource4>(object: TObject, source1: TSource1, source2: TSource2, source3: TSource3, source4: TSource4, customizer: MergeWithCustomizer): TObject & TSource1 & TSource2 & TSource3 & TSource4;
/**
 * Merges the properties of one or more source objects into the target object.
 *
 * @param {any} object - The target object into which the source object properties will be merged.
 * @param {...any} otherArgs - Additional source objects to merge into the target object, including the custom `merge` function.
 * @returns {any} The updated target object with properties from the source object(s) merged in.
 *
 * @example
 * const target = { a: 1, b: 2 };
 * const source = { b: 3, c: 4 };
 *
 * const result = mergeWith(target, source, (objValue, srcValue) => {
 *   if (typeof objValue === 'number' && typeof srcValue === 'number') {
 *     return objValue + srcValue;
 */
declare function mergeWith(object: any, ...otherArgs: any[]): any;

export { mergeWith };
