/**
 * Check whether a value is a symbol.
 *
 * This function can also serve as a type predicate in TypeScript, narrowing the type of the argument to `symbol`.
 *
 * @param {unknown} value The value to check.
 * @returns {value is symbol} Returns `true` if `value` is a symbol, else `false`.
 * @example
 * isSymbol(Symbol.iterator);
 * // => true
 *
 * isSymbol('abc');
 * // => false
 */
declare function isSymbol(value: any): value is symbol;

export { isSymbol };
