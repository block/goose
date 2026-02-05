import { PropertyPath } from '../_internal/PropertyPath.mjs';

/**
 * Updates the value at the specified path of the given object using an updater function.
 * If any part of the path does not exist, it will be created.
 *
 * @param {object} obj - The object to modify.
 * @param {PropertyPath} path - The path of the property to update.
 * @param {(value: any) => any} updater - The function to produce the updated value.
 * @returns {any} - The modified object.
 */
declare function update(obj: object, path: PropertyPath, updater: (value: any) => any): any;

export { update };
