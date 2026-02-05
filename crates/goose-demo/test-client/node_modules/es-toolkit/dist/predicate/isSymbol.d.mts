/**
 * Check whether a value is a symbol.
 *
 * This function can also serve as a type predicate in TypeScript, narrowing the type of the argument to `symbol`.
 *
 * @param {unknown} value The value to check.
 * @returns {value is symbol} Returns `true` if `value` is a symbol, else `false`.
 *
 * @example
 * import { isSymbol } from 'es-toolkit/predicate';
 *
 * isSymbol(Symbol('a')); // true
 * isSymbol(Symbol.for('a')); // true
 * isSymbol(Symbol.iterator); // true
 *
 * isSymbol(null); // false
 * isSymbol(undefined); // false
 * isSymbol('123'); // false
 * isSymbol(false); // false
 * isSymbol(123n); // false
 * isSymbol({}); // false
 * isSymbol([1, 2, 3]); // false
 */
declare function isSymbol(value: unknown): value is symbol;

export { isSymbol };
