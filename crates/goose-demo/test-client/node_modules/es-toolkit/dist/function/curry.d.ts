/**
 * Curries a function, allowing it to be called with a single argument at a time and returning a new function that takes the next argument.
 * This process continues until all arguments have been provided, at which point the original function is called with all accumulated arguments.
 *
 * @param {() => R} func - The function to curry.
 * @returns {() => R} A curried function.
 *
 * @example
 * function noArgFunc() {
 *   return 42;
 * }
 * const curriedNoArgFunc = curry(noArgFunc);
 * console.log(curriedNoArgFunc()); // 42
 */
declare function curry<R>(func: () => R): () => R;
/**
 * Curries a function, allowing it to be called with a single argument at a time and returning a new function that takes the next argument.
 * This process continues until all arguments have been provided, at which point the original function is called with all accumulated arguments.
 *
 * @param {(p: P) => R} func - The function to curry.
 * @returns {(p: P) => R} A curried function.
 *
 * @example
 * function oneArgFunc(a: number) {
 *   return a * 2;
 * }
 * const curriedOneArgFunc = curry(oneArgFunc);
 * console.log(curriedOneArgFunc(5)); // 10
 */
declare function curry<P, R>(func: (p: P) => R): (p: P) => R;
/**
 * Curries a function, allowing it to be called with a single argument at a time and returning a new function that takes the next argument.
 * This process continues until all arguments have been provided, at which point the original function is called with all accumulated arguments.
 *
 * @param {(p1: P1, p2: P2) => R} func - The function to curry.
 * @returns {(p1: P1) => (p2: P2) => R} A curried function.
 *
 * @example
 * function twoArgFunc(a: number, b: number) {
 *   return a + b;
 * }
 * const curriedTwoArgFunc = curry(twoArgFunc);
 * const add5 = curriedTwoArgFunc(5);
 * console.log(add5(10)); // 15
 */
declare function curry<P1, P2, R>(func: (p1: P1, p2: P2) => R): (p1: P1) => (p2: P2) => R;
/**
 * Curries a function, allowing it to be called with a single argument at a time and returning a new function that takes the next argument.
 * This process continues until all arguments have been provided, at which point the original function is called with all accumulated arguments.
 *
 * @param {(p1: P1, p2: P2, p3: P3) => R} func - The function to curry.
 * @returns {(p1: P1) => (p2: P2) => (p3: P3) => R} A curried function.
 *
 * @example
 * function threeArgFunc(a: number, b: number, c: number) {
 *   return a + b + c;
 * }
 * const curriedThreeArgFunc = curry(threeArgFunc);
 * const add1 = curriedThreeArgFunc(1);
 * const add3 = add1(2);
 * console.log(add3(3)); // 6
 */
declare function curry<P1, P2, P3, R>(func: (p1: P1, p2: P2, p3: P3) => R): (p1: P1) => (p2: P2) => (p3: P3) => R;
/**
 * Curries a function, allowing it to be called with a single argument at a time and returning a new function that takes the next argument.
 * This process continues until all arguments have been provided, at which point the original function is called with all accumulated arguments.
 *
 * @param {(p1: P1, p2: P2, p3: P3, p4: P4) => R} func - The function to curry.
 * @returns {(p1: P1) => (p2: P2) => (p3: P3) => (p4: P4) => R} A curried function.
 *
 * @example
 * function fourArgFunc(a: number, b: number, c: number, d: number) {
 *   return a + b + c + d;
 * }
 * const curriedFourArgFunc = curry(fourArgFunc);
 * const add1 = curriedFourArgFunc(1);
 * const add3 = add1(2);
 * const add6 = add3(3);
 * console.log(add6(4)); // 10
 */
declare function curry<P1, P2, P3, P4, R>(func: (p1: P1, p2: P2, p3: P3, p4: P4) => R): (p1: P1) => (p2: P2) => (p3: P3) => (p4: P4) => R;
/**
 * Curries a function, allowing it to be called with a single argument at a time and returning a new function that takes the next argument.
 * This process continues until all arguments have been provided, at which point the original function is called with all accumulated arguments.
 *
 * @param {(p1: P1, p2: P2, p3: P3, p4: P4, p5: P5) => R} func - The function to curry.
 * @returns {(p1: P1) => (p2: P2) => (p3: P3) => (p4: P4) => (p5: P5) => R} A curried function.
 *
 * @example
 * function fiveArgFunc(a: number, b: number, c: number, d: number, e: number) {
 *   return a + b + c + d + e;
 * }
 * const curriedFiveArgFunc = curry(fiveArgFunc);
 * const add1 = curriedFiveArgFunc(1);
 * const add3 = add1(2);
 * const add6 = add3(3);
 * const add10 = add6(4);
 * console.log(add10(5)); // 15
 */
declare function curry<P1, P2, P3, P4, P5, R>(func: (p1: P1, p2: P2, p3: P3, p4: P4, p5: P5) => R): (p1: P1) => (p2: P2) => (p3: P3) => (p4: P4) => (p5: P5) => R;
/**
 * Curries a function, allowing it to be called with a single argument at a time and returning a new function that takes the next argument.
 * This process continues until all arguments have been provided, at which point the original function is called with all accumulated arguments.
 *
 * @param {(...args: any[]) => any} func - The function to curry.
 * @returns {(...args: any[]) => any} A curried function that can be called with a single argument at a time.
 *
 * @example
 * function sum(a: number, b: number, c: number) {
 *   return a + b + c;
 * }
 *
 * const curriedSum = curry(sum);
 *
 * // The parameter `a` should be given the value `10`.
 * const add10 = curriedSum(10);
 *
 * // The parameter `b` should be given the value `15`.
 * const add25 = add10(15);
 *
 * // The parameter `c` should be given the value `5`. The function 'sum' has received all its arguments and will now return a value.
 * const result = add25(5);
 */
declare function curry(func: (...args: any[]) => any): (...args: any[]) => any;

export { curry };
