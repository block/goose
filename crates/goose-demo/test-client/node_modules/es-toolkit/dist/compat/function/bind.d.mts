/**
 * Creates a function that invokes `func` with the `this` binding of `thisArg` and `partials` prepended to the arguments it receives.
 *
 * The `bind.placeholder` value, which defaults to a `symbol`, may be used as a placeholder for partially applied arguments.
 *
 * Note: Unlike native `Function#bind`, this method doesn't set the `length` property of bound functions.
 *
 * @param {(...args: any[]) => any} func - The function to bind.
 * @param {any} thisObj - The `this` binding of `func`.
 * @param {...any} partialArgs - The arguments to be partially applied.
 * @returns {(...args: any[]) => any} - Returns the new bound function.
 *
 * @example
 * function greet(greeting, punctuation) {
 *   return greeting + ' ' + this.user + punctuation;
 * }
 * const object = { user: 'fred' };
 * let bound = bind(greet, object, 'hi');
 * bound('!');
 * // => 'hi fred!'
 *
 * bound = bind(greet, object, bind.placeholder, '!');
 * bound('hi');
 * // => 'hi fred!'
 */
declare function bind(func: (...args: any[]) => any, thisObj: any, ...partialArgs: any[]): (...args: any[]) => any;
declare namespace bind {
    var placeholder: typeof bindPlaceholder;
}
declare const bindPlaceholder: unique symbol;

export { bind };
