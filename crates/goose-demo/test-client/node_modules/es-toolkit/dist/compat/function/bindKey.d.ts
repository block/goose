/**
 * Creates a function that invokes the method at `object[key]` with `partialArgs` prepended to the arguments it receives.
 *
 * This method differs from `bind` by allowing bound functions to reference methods that may be redefined or don't yet exist.
 *
 * The `bindKey.placeholder` value, which defaults to a `symbol`, may be used as a placeholder for partially applied arguments.
 *
 * @template T - The type of the object to bind.
 * @template K - The type of the key to bind.
 * @param {T} object - The object to invoke the method on.
 * @param {K} key - The key of the method.
 * @param {...any} partialArgs - The arguments to be partially applied.
 * @returns {T[K] extends (...args: any[]) => any ? (...args: any[]) => ReturnType<T[K]> : never} - Returns the new bound function.
 *
 * @example
 * const object = {
 *   user: 'fred',
 *   greet: function (greeting, punctuation) {
 *     return greeting + ' ' + this.user + punctuation;
 *   },
 * };
 *
 * let bound = bindKey(object, 'greet', 'hi');
 * bound('!');
 * // => 'hi fred!'
 *
 * object.greet = function (greeting, punctuation) {
 *   return greeting + 'ya ' + this.user + punctuation;
 * };
 *
 * bound('!');
 * // => 'hiya fred!'
 *
 * // Bound with placeholders.
 * bound = bindKey(object, 'greet', bindKey.placeholder, '!');
 * bound('hi');
 * // => 'hiya fred!'
 */
declare function bindKey(object: object, key: string, ...partialArgs: any[]): (...args: any[]) => any;
declare namespace bindKey {
    var placeholder: typeof bindKeyPlaceholder;
}
declare const bindKeyPlaceholder: unique symbol;

export { bindKey };
