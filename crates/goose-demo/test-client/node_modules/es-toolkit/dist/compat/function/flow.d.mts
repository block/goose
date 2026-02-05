import { Many } from '../_internal/Many.mjs';

/**
 * Creates a new function that executes the given functions in sequence. The return value of the previous function is passed as an argument to the next function.
 *
 * @template A - The type of the arguments.
 * @template R - The type of the return values.
 * @param {(...args: A) => R} f1 - The first function to invoke.
 * @param {(a: R) => R} f2 - The second function to invoke.
 * @param {(a: R) => R} f3 - The third function to invoke.
 * @param {(a: R) => R} f4 - The fourth function to invoke.
 * @param {(a: R) => R} f5 - The fifth function to invoke.
 * @param {(a: R) => R} f6 - The sixth function to invoke.
 * @param {(a: R) => R} f7 - The seventh function to invoke.
 * @returns {(...args: A) => R} Returns the new composite function.
 *
 * @example
 * function square(n) {
 *   return n * n;
 * }
 *
 * var addSquare = flow([add, square]);
 * addSquare(1, 2);
 * // => 9
 */
declare function flow<A extends any[], R1, R2, R3, R4, R5, R6, R7>(f1: (...args: A) => R1, f2: (a: R1) => R2, f3: (a: R2) => R3, f4: (a: R3) => R4, f5: (a: R4) => R5, f6: (a: R5) => R6, f7: (a: R6) => R7): (...args: A) => R7;
/**
 * Creates a new function that executes up to 7 functions in sequence, with additional functions flattened.
 * The return value of each function is passed as an argument to the next function.
 *
 * @example
 * const add = (x: number, y: number) => x + y;
 * const square = (n: number) => n * n;
 * const double = (n: number) => n * 2;
 * const toString = (n: number) => n.toString();
 *
 * const combined = flow(add, square, double, toString);
 * console.log(combined(1, 2)); // "18"
 */
declare function flow<A extends any[], R1, R2, R3, R4, R5, R6, R7>(f1: (...args: A) => R1, f2: (a: R1) => R2, f3: (a: R2) => R3, f4: (a: R3) => R4, f5: (a: R4) => R5, f6: (a: R5) => R6, f7: (a: R6) => R7, ...func: Array<Many<(a: any) => any>>): (...args: A) => any;
/**
 * Creates a new function that executes 6 functions in sequence.
 * The return value of each function is passed as an argument to the next function.
 *
 * @example
 * const add = (x: number, y: number) => x + y;
 * const square = (n: number) => n * n;
 * const double = (n: number) => n * 2;
 *
 * const combined = flow(add, square, double);
 * console.log(combined(1, 2)); // 18
 */
declare function flow<A extends any[], R1, R2, R3, R4, R5, R6>(f1: (...args: A) => R1, f2: (a: R1) => R2, f3: (a: R2) => R3, f4: (a: R3) => R4, f5: (a: R4) => R5, f6: (a: R5) => R6): (...args: A) => R6;
/**
 * Creates a new function that executes 5 functions in sequence.
 * The return value of each function is passed as an argument to the next function.
 *
 * @example
 * const add = (x: number, y: number) => x + y;
 * const square = (n: number) => n * n;
 * const double = (n: number) => n * 2;
 *
 * const combined = flow(add, square, double);
 * console.log(combined(1, 2)); // 18
 */
declare function flow<A extends any[], R1, R2, R3, R4, R5>(f1: (...args: A) => R1, f2: (a: R1) => R2, f3: (a: R2) => R3, f4: (a: R3) => R4, f5: (a: R4) => R5): (...args: A) => R5;
/**
 * Creates a new function that executes 4 functions in sequence.
 * The return value of each function is passed as an argument to the next function.
 *
 * @example
 * const add = (x: number, y: number) => x + y;
 * const square = (n: number) => n * n;
 * const double = (n: number) => n * 2;
 *
 * const combined = flow(add, square, double);
 * console.log(combined(1, 2)); // 18
 */
declare function flow<A extends any[], R1, R2, R3, R4>(f1: (...args: A) => R1, f2: (a: R1) => R2, f3: (a: R2) => R3, f4: (a: R3) => R4): (...args: A) => R4;
/**
 * Creates a new function that executes 3 functions in sequence.
 * The return value of each function is passed as an argument to the next function.
 *
 * @example
 * const add = (x: number, y: number) => x + y;
 * const square = (n: number) => n * n;
 * const double = (n: number) => n * 2;
 *
 * const combined = flow(add, square, double);
 * console.log(combined(1, 2)); // 18
 */
declare function flow<A extends any[], R1, R2, R3>(f1: (...args: A) => R1, f2: (a: R1) => R2, f3: (a: R2) => R3): (...args: A) => R3;
/**
 * Creates a new function that executes 2 functions in sequence.
 * The return value of the first function is passed as an argument to the second function.
 *
 * @example
 * const add = (x: number, y: number) => x + y;
 * const square = (n: number) => n * n;
 *
 * const addThenSquare = flow(add, square);
 * console.log(addThenSquare(1, 2)); // 9
 */
declare function flow<A extends any[], R1, R2>(f1: (...args: A) => R1, f2: (a: R1) => R2): (...args: A) => R2;
/**
 * Creates a new function that executes the given functions in sequence.
 * The return value of each function is passed as an argument to the next function.
 *
 * @example
 * const add = (x: number, y: number) => x + y;
 * const square = (n: number) => n * n;
 * const double = (n: number) => n * 2;
 *
 * const combined = flow(add, square, double);
 * console.log(combined(1, 2)); // 18
 */
declare function flow(...func: Array<Many<(...args: any[]) => any>>): (...args: any[]) => any;

export { flow };
