import { GetFieldType } from '../_internal/GetFieldType.js';
import { PropertyPath } from '../_internal/PropertyPath.js';

/**
 * Gets the value at path of object. If the resolved value is undefined, the defaultValue is returned in its place.
 *
 * @template TObject
 * @template TKey
 * @param {TObject} object - The object to query.
 * @param {TKey | [TKey]} path - The path of the property to get.
 * @returns {TObject[TKey]} Returns the resolved value.
 *
 * @example
 * const object = { 'a': [{ 'b': { 'c': 3 } }] };
 * get(object, 'a[0].b.c');
 * // => 3
 */
declare function get<TObject extends object, TKey extends keyof TObject>(object: TObject, path: TKey | [TKey]): TObject[TKey];
/**
 * Gets the value at path of object. If the resolved value is undefined, the defaultValue is returned in its place.
 *
 * @template TObject
 * @template TKey
 * @param {TObject | null | undefined} object - The object to query.
 * @param {TKey | [TKey]} path - The path of the property to get.
 * @returns {TObject[TKey] | undefined} Returns the resolved value.
 *
 * @example
 * const object = { 'a': [{ 'b': { 'c': 3 } }] };
 * get(object, 'a[0].b.c');
 * // => 3
 */
declare function get<TObject extends object, TKey extends keyof TObject>(object: TObject | null | undefined, path: TKey | [TKey]): TObject[TKey] | undefined;
/**
 * Gets the value at path of object. If the resolved value is undefined, the defaultValue is returned in its place.
 *
 * @template TObject
 * @template TKey
 * @template TDefault
 * @param {TObject | null | undefined} object - The object to query.
 * @param {TKey | [TKey]} path - The path of the property to get.
 * @param {TDefault} defaultValue - The value returned if the resolved value is undefined.
 * @returns {Exclude<TObject[TKey], undefined> | TDefault} Returns the resolved value.
 *
 * @example
 * const object = { 'a': [{ 'b': { 'c': 3 } }] };
 * get(object, 'a[0].b.c', 'default');
 * // => 3
 */
declare function get<TObject extends object, TKey extends keyof TObject, TDefault>(object: TObject | null | undefined, path: TKey | [TKey], defaultValue: TDefault): Exclude<TObject[TKey], undefined> | TDefault;
/**
 * Gets the value at path of object. If the resolved value is undefined, the defaultValue is returned in its place.
 *
 * @template TObject
 * @template TKey1
 * @template TKey2
 * @param {TObject} object - The object to query.
 * @param {[TKey1, TKey2]} path - The path of the property to get.
 * @returns {TObject[TKey1][TKey2]} Returns the resolved value.
 *
 * @example
 * const object = { 'a': { 'b': 2 } };
 * get(object, ['a', 'b']);
 * // => 2
 */
declare function get<TObject extends object, TKey1 extends keyof TObject, TKey2 extends keyof TObject[TKey1]>(object: TObject, path: [TKey1, TKey2]): TObject[TKey1][TKey2];
/**
 * Gets the value at path of object. If the resolved value is undefined, the defaultValue is returned in its place.
 *
 * @template TObject
 * @template TKey1
 * @template TKey2
 * @param {TObject | null | undefined} object - The object to query.
 * @param {[TKey1, TKey2]} path - The path of the property to get.
 * @returns {NonNullable<TObject[TKey1]>[TKey2] | undefined} Returns the resolved value.
 *
 * @example
 * const object = { 'a': { 'b': 2 } };
 * get(object, ['a', 'b']);
 * // => 2
 */
declare function get<TObject extends object, TKey1 extends keyof TObject, TKey2 extends keyof NonNullable<TObject[TKey1]>>(object: TObject | null | undefined, path: [TKey1, TKey2]): NonNullable<TObject[TKey1]>[TKey2] | undefined;
/**
 * Gets the value at path of object. If the resolved value is undefined, the defaultValue is returned in its place.
 *
 * @template TObject
 * @template TKey1
 * @template TKey2
 * @template TDefault
 * @param {TObject | null | undefined} object - The object to query.
 * @param {[TKey1, TKey2]} path - The path of the property to get.
 * @param {TDefault} defaultValue - The value returned if the resolved value is undefined.
 * @returns {Exclude<NonNullable<TObject[TKey1]>[TKey2], undefined> | TDefault} Returns the resolved value.
 *
 * @example
 * const object = { 'a': { 'b': 2 } };
 * get(object, ['a', 'b'], 'default');
 * // => 2
 */
declare function get<TObject extends object, TKey1 extends keyof TObject, TKey2 extends keyof NonNullable<TObject[TKey1]>, TDefault>(object: TObject | null | undefined, path: [TKey1, TKey2], defaultValue: TDefault): Exclude<NonNullable<TObject[TKey1]>[TKey2], undefined> | TDefault;
/**
 * Gets the value at path of object. If the resolved value is undefined, the defaultValue is returned in its place.
 *
 * @template TObject
 * @template TKey1
 * @template TKey2
 * @template TKey3
 * @param {TObject} object - The object to query.
 * @param {[TKey1, TKey2, TKey3]} path - The path of the property to get.
 * @returns {TObject[TKey1][TKey2][TKey3]} Returns the resolved value.
 *
 * @example
 * const object = { 'a': { 'b': { 'c': 3 } } };
 * get(object, ['a', 'b', 'c']);
 * // => 3
 */
declare function get<TObject extends object, TKey1 extends keyof TObject, TKey2 extends keyof TObject[TKey1], TKey3 extends keyof TObject[TKey1][TKey2]>(object: TObject, path: [TKey1, TKey2, TKey3]): TObject[TKey1][TKey2][TKey3];
/**
 * Gets the value at path of object. If the resolved value is undefined, the defaultValue is returned in its place.
 *
 * @template TObject
 * @template TKey1
 * @template TKey2
 * @template TKey3
 * @param {TObject | null | undefined} object - The object to query.
 * @param {[TKey1, TKey2, TKey3]} path - The path of the property to get.
 * @returns {NonNullable<NonNullable<TObject[TKey1]>[TKey2]>[TKey3] | undefined} Returns the resolved value.
 *
 * @example
 * const object = { 'a': { 'b': { 'c': 3 } } };
 * get(object, ['a', 'b', 'c']);
 * // => 3
 */
declare function get<TObject extends object, TKey1 extends keyof TObject, TKey2 extends keyof NonNullable<TObject[TKey1]>, TKey3 extends keyof NonNullable<NonNullable<TObject[TKey1]>[TKey2]>>(object: TObject | null | undefined, path: [TKey1, TKey2, TKey3]): NonNullable<NonNullable<TObject[TKey1]>[TKey2]>[TKey3] | undefined;
/**
 * Gets the value at path of object. If the resolved value is undefined, the defaultValue is returned in its place.
 *
 * @template TObject
 * @template TKey1
 * @template TKey2
 * @template TKey3
 * @template TDefault
 * @param {TObject | null | undefined} object - The object to query.
 * @param {[TKey1, TKey2, TKey3]} path - The path of the property to get.
 * @param {TDefault} defaultValue - The value returned if the resolved value is undefined.
 * @returns {Exclude<NonNullable<NonNullable<TObject[TKey1]>[TKey2]>[TKey3], undefined> | TDefault} Returns the resolved value.
 *
 * @example
 * const object = { 'a': { 'b': { 'c': 3 } } };
 * get(object, ['a', 'b', 'c'], 'default');
 * // => 3
 */
declare function get<TObject extends object, TKey1 extends keyof TObject, TKey2 extends keyof NonNullable<TObject[TKey1]>, TKey3 extends keyof NonNullable<NonNullable<TObject[TKey1]>[TKey2]>, TDefault>(object: TObject | null | undefined, path: [TKey1, TKey2, TKey3], defaultValue: TDefault): Exclude<NonNullable<NonNullable<TObject[TKey1]>[TKey2]>[TKey3], undefined> | TDefault;
/**
 * Gets the value at path of object.
 *
 * @template TObject
 * @template TKey1
 * @template TKey2
 * @template TKey3
 * @template TKey4
 * @param {TObject} object - The object to query.
 * @param {[TKey1, TKey2, TKey3, TKey4]} path - The path of the property to get.
 * @returns {TObject[TKey1][TKey2][TKey3][TKey4]} Returns the resolved value.
 *
 * @example
 * const object = { 'a': { 'b': { 'c': { 'd': 4 } } } };
 * get(object, ['a', 'b', 'c', 'd']);
 * // => 4
 */
declare function get<TObject extends object, TKey1 extends keyof TObject, TKey2 extends keyof TObject[TKey1], TKey3 extends keyof TObject[TKey1][TKey2], TKey4 extends keyof TObject[TKey1][TKey2][TKey3]>(object: TObject, path: [TKey1, TKey2, TKey3, TKey4]): TObject[TKey1][TKey2][TKey3][TKey4];
/**
 * Gets the value at path of object. If the resolved value is undefined, undefined is returned.
 *
 * @template TObject
 * @template TKey1
 * @template TKey2
 * @template TKey3
 * @template TKey4
 * @param {TObject | null | undefined} object - The object to query.
 * @param {[TKey1, TKey2, TKey3, TKey4]} path - The path of the property to get.
 * @returns {NonNullable<NonNullable<NonNullable<TObject[TKey1]>[TKey2]>[TKey3]>[TKey4] | undefined} Returns the resolved value.
 *
 * @example
 * const object = { 'a': { 'b': { 'c': { 'd': 4 } } } };
 * get(object, ['a', 'b', 'c', 'd']);
 * // => 4
 */
declare function get<TObject extends object, TKey1 extends keyof TObject, TKey2 extends keyof NonNullable<TObject[TKey1]>, TKey3 extends keyof NonNullable<NonNullable<TObject[TKey1]>[TKey2]>, TKey4 extends keyof NonNullable<NonNullable<NonNullable<TObject[TKey1]>[TKey2]>[TKey3]>>(object: TObject | null | undefined, path: [TKey1, TKey2, TKey3, TKey4]): NonNullable<NonNullable<NonNullable<TObject[TKey1]>[TKey2]>[TKey3]>[TKey4] | undefined;
/**
 * Gets the value at path of object. If the resolved value is undefined, the defaultValue is returned in its place.
 *
 * @template TObject
 * @template TKey1
 * @template TKey2
 * @template TKey3
 * @template TKey4
 * @template TDefault
 * @param {TObject | null | undefined} object - The object to query.
 * @param {[TKey1, TKey2, TKey3, TKey4]} path - The path of the property to get.
 * @param {TDefault} defaultValue - The value returned if the resolved value is undefined.
 * @returns {Exclude<NonNullable<NonNullable<NonNullable<TObject[TKey1]>[TKey2]>[TKey3]>[TKey4], undefined> | TDefault} Returns the resolved value.
 *
 * @example
 * const object = { 'a': { 'b': { 'c': { 'd': 4 } } } };
 * get(object, ['a', 'b', 'c', 'd'], 'default');
 * // => 4
 */
declare function get<TObject extends object, TKey1 extends keyof TObject, TKey2 extends keyof NonNullable<TObject[TKey1]>, TKey3 extends keyof NonNullable<NonNullable<TObject[TKey1]>[TKey2]>, TKey4 extends keyof NonNullable<NonNullable<NonNullable<TObject[TKey1]>[TKey2]>[TKey3]>, TDefault>(object: TObject | null | undefined, path: [TKey1, TKey2, TKey3, TKey4], defaultValue: TDefault): Exclude<NonNullable<NonNullable<NonNullable<TObject[TKey1]>[TKey2]>[TKey3]>[TKey4], undefined> | TDefault;
/**
 * Gets the value at path of object.
 *
 * @template T
 * @param {Record<number, T>} object - The object to query.
 * @param {number} path - The path of the property to get.
 * @returns {T} Returns the resolved value.
 *
 * @example
 * const object = { 0: 'a', 1: 'b', 2: 'c' };
 * get(object, 1);
 * // => 'b'
 */
declare function get<T>(object: Record<number, T>, path: number): T;
/**
 * Gets the value at path of object. If the resolved value is undefined, undefined is returned.
 *
 * @template T
 * @param {Record<number, T> | null | undefined} object - The object to query.
 * @param {number} path - The path of the property to get.
 * @returns {T | undefined} Returns the resolved value.
 *
 * @example
 * const object = { 0: 'a', 1: 'b', 2: 'c' };
 * get(object, 1);
 * // => 'b'
 */
declare function get<T>(object: Record<number, T> | null | undefined, path: number): T | undefined;
/**
 * Gets the value at path of object. If the resolved value is undefined, the defaultValue is returned in its place.
 *
 * @template T
 * @template TDefault
 * @param {Record<number, T> | null | undefined} object - The object to query.
 * @param {number} path - The path of the property to get.
 * @param {TDefault} defaultValue - The value returned if the resolved value is undefined.
 * @returns {T | TDefault} Returns the resolved value.
 *
 * @example
 * const object = { 0: 'a', 1: 'b', 2: 'c' };
 * get(object, 1, 'default');
 * // => 'b'
 */
declare function get<T, TDefault>(object: Record<number, T> | null | undefined, path: number, defaultValue: TDefault): T | TDefault;
/**
 * Gets the value at path of object. If the resolved value is undefined, the defaultValue is returned in its place.
 *
 * @template TDefault
 * @param {null | undefined} object - The object to query.
 * @param {PropertyPath} path - The path of the property to get.
 * @param {TDefault} defaultValue - The value returned if the resolved value is undefined.
 * @returns {TDefault} Returns the default value.
 *
 * @example
 * get(null, 'a.b.c', 'default');
 * // => 'default'
 */
declare function get<TDefault>(object: null | undefined, path: PropertyPath, defaultValue: TDefault): TDefault;
/**
 * Gets the value at path of object. If the resolved value is undefined, undefined is returned.
 *
 * @param {null | undefined} object - The object to query.
 * @param {PropertyPath} path - The path of the property to get.
 * @returns {undefined} Returns undefined.
 *
 * @example
 * get(null, 'a.b.c');
 * // => undefined
 */
declare function get(object: null | undefined, path: PropertyPath): undefined;
/**
 * Gets the value at path of object using type-safe path.
 *
 * @template TObject
 * @template TPath
 * @param {TObject} data - The object to query.
 * @param {TPath} path - The path of the property to get.
 * @returns {string extends TPath ? any : GetFieldType<TObject, TPath>} Returns the resolved value.
 *
 * @example
 * const object = { a: { b: { c: 1 } } };
 * get(object, 'a.b.c');
 * // => 1
 */
declare function get<TObject, TPath extends string>(data: TObject, path: TPath): string extends TPath ? any : GetFieldType<TObject, TPath>;
/**
 * Gets the value at path of object using type-safe path. If the resolved value is undefined, the defaultValue is returned.
 *
 * @template TObject
 * @template TPath
 * @template TDefault
 * @param {TObject} data - The object to query.
 * @param {TPath} path - The path of the property to get.
 * @param {TDefault} defaultValue - The value returned if the resolved value is undefined.
 * @returns {Exclude<GetFieldType<TObject, TPath>, null | undefined> | TDefault} Returns the resolved value.
 *
 * @example
 * const object = { a: { b: { c: 1 } } };
 * get(object, 'a.b.d', 'default');
 * // => 'default'
 */
declare function get<TObject, TPath extends string, TDefault = GetFieldType<TObject, TPath>>(data: TObject, path: TPath, defaultValue: TDefault): Exclude<GetFieldType<TObject, TPath>, null | undefined> | TDefault;
/**
 * Gets the value at path of object. If the resolved value is undefined, the defaultValue is returned.
 *
 * @param {any} object - The object to query.
 * @param {PropertyPath} path - The path of the property to get.
 * @param {any} [defaultValue] - The value returned if the resolved value is undefined.
 * @returns {any} Returns the resolved value.
 *
 * @example
 * const object = { a: { b: { c: 1 } } };
 * get(object, 'a.b.c', 'default');
 * // => 1
 */
declare function get(object: any, path: PropertyPath, defaultValue?: any): any;

export { get };
