import { PropertyPath } from '../_internal/PropertyPath.js';

/**
 * Updates the value at the specified path of the given object using an updater function and a customizer.
 * If any part of the path does not exist, it will be created.
 *
 * @template T - The type of the object.
 * @param {T} object - The object to modify.
 * @param {PropertyPath} path - The path of the property to update.
 * @param {(oldValue: any) => any} updater - The function to produce the updated value.
 * @param {(value: any, key: string, object: T) => any} customizer - The function to customize the update process.
 * @returns {T} - The modified object.
 *
 * @example
 * const object = { 'a': [{ 'b': { 'c': 3 } }] };
 * updateWith(object, 'a[0].b.c', (n) => n * n);
 * // => { 'a': [{ 'b': { 'c': 9 } }] }
 */
declare function updateWith<T extends object>(object: T, path: PropertyPath, updater: (oldValue: any) => any, customizer?: (value: any, key: string, object: T) => any): T;
/**
 * Updates the value at the specified path of the given object using an updater function and a customizer.
 * If any part of the path does not exist, it will be created.
 *
 * @template T - The type of the object.
 * @template R - The type of the return value.
 * @param {T} object - The object to modify.
 * @param {PropertyPath} path - The path of the property to update.
 * @param {(oldValue: any) => any} updater - The function to produce the updated value.
 * @param {(value: any, key: string, object: T) => any} customizer - The function to customize the update process.
 * @returns {R} - The modified object.
 *
 * @example
 * const object = { 'a': [{ 'b': { 'c': 3 } }] };
 * updateWith(object, 'a[0].b.c', (n) => n * n);
 * // => { 'a': [{ 'b': { 'c': 9 } }] }
 */
declare function updateWith<T extends object, R>(object: T, path: PropertyPath, updater: (oldValue: any) => any, customizer?: (value: any, key: string, object: T) => any): R;

export { updateWith };
