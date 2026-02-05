/**
 * Creates a new function that executes the given functions in sequence from right to left. The return value of the previous function is passed as an argument to the next function.
 *
 * The `this` context of the returned function is also passed to the functions provided as parameters.
 *
 * This method is like `flow` except that it creates a function that invokes the given functions from right to left.
 *
 * @param {() => R} f The function to invoke.
 * @returns {() => R} Returns the new composite function.
 *
 * @example
 * function noArgFunc() {
 *   return 42;
 * }
 * const combined = flowRight(noArgFunc);
 * console.log(combined()); // 42
 */
declare function flowRight<R>(f: () => R): () => R;
/**
 * Creates a new function that executes the given functions in sequence from right to left. The return value of the previous function is passed as an argument to the next function.
 *
 * The `this` context of the returned function is also passed to the functions provided as parameters.
 *
 * This method is like `flow` except that it creates a function that invokes the given functions from right to left.
 *
 * @param {(...args: A) => R} f1 The function to invoke.
 * @returns {(...args: A) => R} Returns the new composite function.
 *
 * @example
 * function oneArgFunc(a: number) {
 *  return a * 2;
 * }
 * const combined = flowRight(oneArgFunc);
 * console.log(combined(5)); // 10
 */
declare function flowRight<A extends any[], R>(f1: (...args: A) => R): (...args: A) => R;
/**
 * Creates a new function that executes the given functions in sequence from right to left. The return value of the previous function is passed as an argument to the next function.
 *
 * The `this` context of the returned function is also passed to the functions provided as parameters.
 *
 * This method is like `flow` except that it creates a function that invokes the given functions from right to left.
 *
 * @param {(a: R1) => R2} f2 The function to invoke.
 * @param {(...args: A) => R1} f1 The function to invoke.
 * @returns {(...args: A) => R2} Returns the new composite function.
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
 * Creates a new function that executes the given functions in sequence from right to left. The return value of the previous function is passed as an argument to the next function.
 *
 * The `this` context of the returned function is also passed to the functions provided as parameters.
 *
 * This method is like `flow` except that it creates a function that invokes the given functions from right to left.
 *
 * @param {(a: R2) => R3} f3 The function to invoke.
 * @param {(a: R1) => R2} f2 The function to invoke.
 * @param {(...args: A) => R1} f1 The function to invoke.
 * @returns {(...args: A) => R3} Returns the new composite function.
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
 * Creates a new function that executes the given functions in sequence from right to left. The return value of the previous function is passed as an argument to the next function.
 *
 * The `this` context of the returned function is also passed to the functions provided as parameters.
 *
 * This method is like `flow` except that it creates a function that invokes the given functions from right to left.
 *
 * @param {(a: R3) => R4} f4 The function to invoke.
 * @param {(a: R2) => R3} f3 The function to invoke.
 * @param {(a: R1) => R2} f2 The function to invoke.
 * @param {(...args: A) => R1} f1 The function to invoke.
 * @returns {(...args: A) => R4} Returns the new composite function.
 *
 * @example
 * const add = (x: number, y: number) => x + y;
 * const square = (n: number) => n * n;
 * const double = (n: number) => n * 2;
 * const toStr = (n: number) => n.toString();
 *
 * const combined = flowRight(toStr, double, square, add);
 * console.log(combined(1, 2));  // '18'
 */
declare function flowRight<A extends any[], R1, R2, R3, R4>(f4: (a: R3) => R4, f3: (a: R2) => R3, f2: (a: R1) => R2, f1: (...args: A) => R1): (...args: A) => R4;
/**
 * Creates a new function that executes the given functions in sequence from right to left. The return value of the previous function is passed as an argument to the next function.
 *
 * The `this` context of the returned function is also passed to the functions provided as parameters.
 *
 * This method is like `flow` except that it creates a function that invokes the given functions from right to left.
 *
 * @param {(a: R4) => R5} f5 The function to invoke.
 * @param {(a: R3) => R4} f4 The function to invoke.
 * @param {(a: R2) => R3} f3 The function to invoke.
 * @param {(a: R1) => R2} f2 The function to invoke.
 * @param {(...args: A) => R1} f1 The function to invoke.
 * @returns {(...args: A) => R5} Returns the new composite function.
 *
 * @example
 * const add = (x: number, y: number) => x + y;
 * const square = (n: number) => n * n;
 * const double = (n: number) => n * 2;
 * const toStr = (n: number) => n.toString();
 * const split = (s: string) => s.split('');
 *
 * const combined = flowRight(split, toStr, double, square, add);
 * console.log(combined(1, 2)); // ['1', '8']
 */
declare function flowRight<A extends any[], R1, R2, R3, R4, R5>(f5: (a: R4) => R5, f4: (a: R3) => R4, f3: (a: R2) => R3, f2: (a: R1) => R2, f1: (...args: A) => R1): (...args: A) => R5;
/**
 * Creates a new function that executes the given functions in sequence from right to left. The return value of the previous function is passed as an argument to the next function.
 *
 * The `this` context of the returned function is also passed to the functions provided as parameters.
 *
 * This method is like `flow` except that it creates a function that invokes the given functions from right to left.
 *
 * @param {(...args: any[]) => any} funcs The functions to invoke.
 * @returns {(...args: any[]) => any} Returns the new composite function.
 *
 * @example
 * const add = (x: number, y: number) => x + y;
 * const square = (n: number) => n * n;
 *
 * const combined = flowRight(square, add);
 * console.log(combined(1, 2)); // 9
 */
declare function flowRight(...funcs: Array<(...args: any[]) => any>): (...args: any[]) => any;

export { flowRight };
