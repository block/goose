/**
 * Creates a new function that spreads elements of an array argument into individual arguments
 * for the original function.
 *
 * @template F - A function type with any number of parameters and any return type.
 * @param {F} func - The function to be transformed. It can be any function with any number of arguments.
 * @returns {(argsArr: Parameters<F>) => ReturnType<F>} - A new function that takes an array of arguments and returns the result of calling the original function with those arguments.
 *
 * @example
 * function add(a, b) {
 *   return a + b;
 * }
 *
 * const spreadAdd = spread(add);
 * console.log(spreadAdd([1, 2])); // Output: 3
 */
declare function spread<F extends (...args: any[]) => any>(func: F): (argsArr: Parameters<F>) => ReturnType<F>;

export { spread };
