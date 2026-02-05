import { Many } from '../_internal/Many.mjs';

/**
 * Creates a new function that executes the given functions in sequence from right to left. The return value of the previous function is passed as an argument to the next function.
 *
 * @template A - The type of the arguments.
 * @template R - The type of the return values.
 * @param {(a: R) => R} f7 - The seventh function to invoke.
 * @param {(a: R) => R} f6 - The sixth function to invoke.
 * @param {(a: R) => R} f5 - The fifth function to invoke.
 * @param {(a: R) => R} f4 - The fourth function to invoke.
 * @param {(a: R) => R} f3 - The third function to invoke.
 * @param {(a: R) => R} f2 - The second function to invoke.
 * @param {(...args: A) => R} f1 - The first function to invoke.
 * @returns {(...args: A) => R} Returns the new composite function.
 *
 * @example
 * function square(n) {
 *   return n * n;
 * }
 *
 * var addSquare = flowRight(square, add);
 * addSquare(1, 2);
 * // => 9
 */
declare function flowRight<A extends any[], R1, R2, R3, R4, R5, R6, R7>(f7: (a: R6) => R7, f6: (a: R5) => R6, f5: (a: R4) => R5, f4: (a: R3) => R4, f3: (a: R2) => R3, f2: (a: R1) => R2, f1: (...args: A) => R1): (...args: A) => R7;
/**
 * Creates a new function that executes 6 functions in sequence from right to left.
 * The return value of each function is passed as an argument to the next function.
 *
 * @example
 * const add = (x: number, y: number) => x + y;
 * const square = (n: number) => n * n;
 * const double = (n: number) => n * 2;
 * const toString = (n: number) => String(n);
 * const append = (s: string) => s + '!';
 * const length = (s: string) => s.length;
 *
 * const combined = flowRight(length, append, toString, double, square, add);
 * console.log(combined(1, 2)); // 7
 */
declare function flowRight<A extends any[], R1, R2, R3, R4, R5, R6>(f6: (a: R5) => R6, f5: (a: R4) => R5, f4: (a: R3) => R4, f3: (a: R2) => R3, f2: (a: R1) => R2, f1: (...args: A) => R1): (...args: A) => R6;
/**
 * Creates a new function that executes 5 functions in sequence from right to left.
 * The return value of each function is passed as an argument to the next function.
 *
 * @example
 * const add = (x: number, y: number) => x + y;
 * const square = (n: number) => n * n;
 * const double = (n: number) => n * 2;
 * const toString = (n: number) => String(n);
 * const append = (s: string) => s + '!';
 *
 * const combined = flowRight(append, toString, double, square, add);
 * console.log(combined(1, 2)); // '18!'
 */
declare function flowRight<A extends any[], R1, R2, R3, R4, R5>(f5: (a: R4) => R5, f4: (a: R3) => R4, f3: (a: R2) => R3, f2: (a: R1) => R2, f1: (...args: A) => R1): (...args: A) => R5;
/**
 * Creates a new function that executes 4 functions in sequence from right to left.
 * The return value of each function is passed as an argument to the next function.
 *
 * @example
 * const add = (x: number, y: number) => x + y;
 * const square = (n: number) => n * n;
 * const double = (n: number) => n * 2;
 * const toString = (n: number) => String(n);
 *
 * const combined = flowRight(toString, double, square, add);
 * console.log(combined(1, 2)); // '18'
 */
declare function flowRight<A extends any[], R1, R2, R3, R4>(f4: (a: R3) => R4, f3: (a: R2) => R3, f2: (a: R1) => R2, f1: (...args: A) => R1): (...args: A) => R4;
/**
 * Creates a new function that executes 3 functions in sequence from right to left.
 * The return value of each function is passed as an argument to the next function.
 *
 * @example
 * const add = (x: number, y: number) => x + y;
 * const square = (n: number) => n * n;
 * const double = (n: number) => n * 2;
 *
 * const combined = flowRight(double, square, add);
 * console.log(combined(1, 2)); // 18
 */
declare function flowRight<A extends any[], R1, R2, R3>(f3: (a: R2) => R3, f2: (a: R1) => R2, f1: (...args: A) => R1): (...args: A) => R3;
/**
 * Creates a new function that executes 2 functions in sequence from right to left.
 * The return value of the first function is passed as an argument to the second function.
 *
 * @example
 * const add = (x: number, y: number) => x + y;
 * const square = (n: number) => n * n;
 *
 * const combined = flowRight(square, add);
 * console.log(combined(1, 2)); // 9
 */
declare function flowRight<A extends any[], R1, R2>(f2: (a: R1) => R2, f1: (...args: A) => R1): (...args: A) => R2;
/**
 * Creates a new function that executes the given functions in sequence from right to left.
 * The return value of each function is passed as an argument to the next function.
 *
 * @example
 * const add = (x: number, y: number) => x + y;
 * const square = (n: number) => n * n;
 * const double = (n: number) => n * 2;
 * const toString = (n: number) => String(n);
 *
 * // Pass functions as separate arguments
 * const combined1 = flowRight(toString, double, square, add);
 * console.log(combined1(1, 2)); // '18'
 *
 * // Pass functions as arrays
 * const combined2 = flowRight([toString, double], [square, add]);
 * console.log(combined2(1, 2)); // '18'
 */
declare function flowRight(...func: Array<Many<(...args: any[]) => any>>): (...args: any[]) => any;

export { flowRight };
