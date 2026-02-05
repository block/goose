import { Many } from '../_internal/Many.js';

/**
 * Binds methods of an object to the object itself, overwriting the existing method.
 * Method names may be specified as individual arguments or as arrays of method names.
 *
 * @template T - The type of the object.
 * @param {T} object - The object to bind methods to.
 * @param {Array<Many<string>>} [methodNames] - The method names to bind, specified individually or in arrays.
 * @returns {T} - Returns the object.
 *
 * @example
 * const view = {
 *   'label': 'docs',
 *   'click': function() {
 *     console.log('clicked ' + this.label);
 *   }
 * };
 *
 * bindAll(view, ['click']);
 * jQuery(element).on('click', view.click);
 * // => Logs 'clicked docs' when clicked.
 *
 * @example
 * // Using individual method names
 * bindAll(view, 'click');
 * // => Same as above
 */
declare function bindAll<T>(object: T, ...methodNames: Array<Many<string>>): T;

export { bindAll };
