/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * The partial.placeholder value, which defaults to a `symbol`, may be used as a placeholder for partially applied arguments.
 *
 * Note: This method doesn't set the `length` property of partially applied functions.
 *
 * @template T1 The type of the first argument.
 * @template R The return type of the function.
 * @param {function(arg1: T1): R} func The function to partially apply.
 * @param {T1} arg1 The first argument to apply.
 * @returns {function(): R} A new function that takes no arguments and returns the result of the original function.
 *
 * @example
 * const addOne = (x: number) => x + 1;
 * const addOneToFive = partial(addOne, 5);
 * console.log(addOneToFive()); // => 6
 */
declare function partial<T1, R>(func: (arg1: T1) => R, arg1: T1): () => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * The partial.placeholder value, which defaults to a `symbol`, may be used as a placeholder for partially applied arguments.
 *
 * Note: This method doesn't set the `length` property of partially applied functions.
 *
 * @template T1 The type of the first argument.
 * @template T2 The type of the second argument.
 * @template R The return type of the function.
 * @param {function(arg1: T1, arg2: T2): R} func The function to partially apply.
 * @param {T1} arg1 The first argument to apply.
 * @returns {function(arg2: T2): R} A new function that takes the second argument and returns the result of the original function.
 *
 * @example
 * const multiply = (x: number, y: number) => x * y;
 * const double = partial(multiply, 2);
 * console.log(double(5)); // => 10
 */
declare function partial<T1, T2, R>(func: (arg1: T1, arg2: T2) => R, arg1: T1): (arg2: T2) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * The partial.placeholder value, which defaults to a `symbol`, may be used as a placeholder for partially applied arguments.
 *
 * Note: This method doesn't set the `length` property of partially applied functions.
 *
 * @template T1 The type of the first argument.
 * @template T2 The type of the second argument.
 * @template R The return type of the function.
 * @param {function(arg1: T1, arg2: T2): R} func The function to partially apply.
 * @param {Placeholder} placeholder The placeholder for the first argument.
 * @param {T2} arg2 The second argument to apply.
 * @returns {function(arg1: T1): R} A new function that takes the first argument and returns the result of the original function.
 *
 * @example
 * const greet = (greeting: string, name: string) => `${greeting}, ${name}!`;
 * const greetWithHello = partial(greet, partial.placeholder, 'John');
 * console.log(greetWithHello('Hello')); // => 'Hello, John!'
 */
declare function partial<T1, T2, R>(func: (arg1: T1, arg2: T2) => R, placeholder: Placeholder, arg2: T2): (arg1: T1) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * The partial.placeholder value, which defaults to a `symbol`, may be used as a placeholder for partially applied arguments.
 *
 * Note: This method doesn't set the `length` property of partially applied functions.
 *
 * @template T1 The type of the first argument.
 * @template T2 The type of the second argument.
 * @template T3 The type of the third argument.
 * @template R The return type of the function.
 * @param {function(arg1: T1, arg2: T2, arg3: T3): R} func The function to partially apply.
 * @param {T1} arg1 The first argument to apply.
 * @returns {function(arg2: T2, arg3: T3): R} A new function that takes the second and third arguments and returns the result of the original function.
 *
 * @example
 * const sumThree = (a: number, b: number, c: number) => a + b + c;
 * const addFive = partial(sumThree, 5);
 * console.log(addFive(3, 2)); // => 10
 */
declare function partial<T1, T2, T3, R>(func: (arg1: T1, arg2: T2, arg3: T3) => R, arg1: T1): (arg2: T2, arg3: T3) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * The partial.placeholder value, which defaults to a `symbol`, may be used as a placeholder for partially applied arguments.
 *
 * Note: This method doesn't set the `length` property of partially applied functions.
 *
 * @template T1 The type of the first argument.
 * @template T2 The type of the second argument.
 * @template T3 The type of the third argument.
 * @template R The return type of the function.
 * @param {function(arg1: T1, arg2: T2, arg3: T3): R} func The function to partially apply.
 * @param {Placeholder} arg1 The placeholder for the first argument.
 * @param {T2} arg2 The second argument to apply.
 * @returns {function(arg1: T1, arg3: T3): R} A new function that takes the first and third arguments and returns the result of the original function.
 *
 * @example
 * const greet = (greeting: string, name: string) => `${greeting}, ${name}!`;
 * const greetWithPlaceholder = partial(greet, partial.placeholder, 'John');
 * console.log(greetWithPlaceholder('Hello')); // => 'Hello, John!'
 */
declare function partial<T1, T2, T3, R>(func: (arg1: T1, arg2: T2, arg3: T3) => R, arg1: Placeholder, arg2: T2): (arg1: T1, arg3: T3) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * The partial.placeholder value, which defaults to a `symbol`, may be used as a placeholder for partially applied arguments.
 *
 * Note: This method doesn't set the `length` property of partially applied functions.
 *
 * @template T1 The type of the first argument.
 * @template T2 The type of the second argument.
 * @template T3 The type of the third argument.
 * @template R The return type of the function.
 * @param {function(arg1: T1, arg2: T2, arg3: T3): R} func The function to partially apply.
 * @param {Placeholder} arg1 The placeholder for the first argument.
 * @param {Placeholder} arg2 The placeholder for the second argument.
 * @param {T3} arg3 The third argument to apply.
 * @returns {function(arg1: T1, arg2: T2): R} A new function that takes the first and second arguments and returns the result of the original function.
 *
 * @example
 * const multiply = (x: number, y: number, z: number) => x * y * z;
 * const multiplyWithPlaceholders = partial(multiply, partial.placeholder, partial.placeholder, 2);
 * console.log(multiplyWithPlaceholders(3, 4)); // => 24
 */
declare function partial<T1, T2, T3, R>(func: (arg1: T1, arg2: T2, arg3: T3) => R, arg1: Placeholder, arg2: Placeholder, arg3: T3): (arg1: T1, arg2: T2) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * The partial.placeholder value, which defaults to a `symbol`, may be used as a placeholder for partially applied arguments.
 *
 * Note: This method doesn't set the `length` property of partially applied functions.
 *
 * @template T1 The type of the first argument.
 * @template T2 The type of the second argument.
 * @template T3 The type of the third argument.
 * @template R The return type of the function.
 * @param {function(arg1: T1, arg2: T2, arg3: T3): R} func The function to partially apply.
 * @param {T1} arg1 The first argument to apply.
 * @param {Placeholder} arg2 The placeholder for the second argument.
 * @param {T3} arg3 The third argument to apply.
 * @returns {function(arg2: T2): R} A new function that takes the second argument and returns the result of the original function.
 *
 * @example
 * const greet = (greeting: string, name: string) => `${greeting}, ${name}!`;
 * const greetWithPlaceholder = partial(greet, 'Hello', partial.placeholder);
 * console.log(greetWithPlaceholder('John')); // => 'Hello, John!'
 */
declare function partial<T1, T2, T3, R>(func: (arg1: T1, arg2: T2, arg3: T3) => R, arg1: T1, arg2: Placeholder, arg3: T3): (arg2: T2) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * The partial.placeholder value, which defaults to a `symbol`, may be used as a placeholder for partially applied arguments.
 *
 * Note: This method doesn't set the `length` property of partially applied functions.
 *
 * @template T1 The type of the first argument.
 * @template T2 The type of the second argument.
 * @template T3 The type of the third argument.
 * @template R The return type of the function.
 * @param {function(arg1: T1, arg2: T2, arg3: T3): R} func The function to partially apply.
 * @param {Placeholder} arg1 The first argument to apply.
 * @param {T2} arg2 The placeholder for the second argument.
 * @param {T3} arg3 The third argument to apply.
 * @returns {function(arg2: T2): R} A new function that takes the second argument and returns the result of the original function.
 *
 * @example
 * const greet = (greeting: string, name: string) => `${greeting}, ${name}!`;
 * const greetWithPlaceholder = partial(greet, 'Hello', partial.placeholder);
 * console.log(greetWithPlaceholder('John')); // => 'Hello, John!'
 */
declare function partial<T1, T2, T3, R>(func: (arg1: T1, arg2: T2, arg3: T3) => R, plc1: Placeholder, arg2: T2, arg3: T3): (arg1: T1) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * The partial.placeholder value, which defaults to a `symbol`, may be used as a placeholder for partially applied arguments.
 *
 * Note: This method doesn't set the `length` property of partially applied functions.
 *
 * @template T1 The type of the first argument.
 * @template T2 The type of the second argument.
 * @template T3 The type of the third argument.
 * @template T4 The type of the fourth argument.
 * @template R The return type of the function.
 * @param {function(arg1: T1, arg2: T2, arg3: T3, arg4: T4): R} func The function to partially apply.
 * @param {T1} arg1 The first argument to apply.
 * @returns {function(arg2: T2): R} A new function that takes the second argument and returns the result of the original function.
 *
 * @example
 * const multiply = (x: number, y: number, z: number, w: number) => x * y * z * w;
 * const double = partial(multiply, 2);
 * console.log(double(5, 4, 3)); // => 120
 */
declare function partial<T1, T2, T3, T4, R>(func: (arg1: T1, arg2: T2, arg3: T3, arg4: T4) => R, arg1: T1): (arg2: T2, arg3: T3, arg4: T4) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * The partial.placeholder value, which defaults to a `symbol`, may be used as a placeholder for partially applied arguments.
 *
 * Note: This method doesn't set the `length` property of partially applied functions.
 *
 * @template T1 The type of the first argument.
 * @template T2 The type of the second argument.
 * @template T3 The type of the third argument.
 * @template T4 The type of the fourth argument.
 * @template R The return type of the function.
 * @param {function(arg1: T1, arg2: T2, arg3: T3, arg4: T4): R} func The function to partially apply.
 * @param {Placeholder} arg1 The placeholder for the first argument.
 * @param {Placeholder} arg2 The placeholder for the second argument.
 * @param {T3} arg3 The third argument to apply.
 * @param {T4} arg4 The fourth argument to apply.
 * @returns {function(arg1: T1, arg2: T2): R} A new function that takes the first and second arguments and returns the result of the original function.
 *
 * @example
 * const multiply = (x: number, y: number, z: number, w: number) => x * y * z * w;
 * const multiplyWithPlaceholders = partial(multiply, partial.placeholder, partial.placeholder, 2, 3);
 * console.log(multiplyWithPlaceholders(4, 5)); // => 120
 */
declare function partial<T1, T2, T3, T4, R>(func: (arg1: T1, arg2: T2, arg3: T3, arg4: T4) => R, arg1: Placeholder, arg2: Placeholder, arg3: T3, arg4: T4): (arg1: T1, arg2: T2) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * The partial.placeholder value, which defaults to a `symbol`, may be used as a placeholder for partially applied arguments.
 *
 * Note: This method doesn't set the `length` property of partially applied functions.
 *
 * @template T1 The type of the first argument.
 * @template T2 The type of the second argument.
 * @template T3 The type of the third argument.
 * @template T4 The type of the fourth argument.
 * @template R The return type of the function.
 * @param {function(arg1: T1, arg2: T2, arg3: T3, arg4: T4): R} func The function to partially apply.
 * @param {T1} arg1 The first argument to apply.
 * @param {T2} arg2 The second argument to apply.
 * @returns {function(arg3: T3, arg4: T4): R} A new function that takes the third and fourth arguments and returns the result of the original function.
 *
 * @example
 * const sumFour = (a: number, b: number, c: number, d: number) => a + b + c + d;
 * const addOneAndTwo = partial(sumFour, 1, 2);
 * console.log(addOneAndTwo(3, 4)); // => 10
 */
declare function partial<T1, T2, T3, T4, R>(func: (arg1: T1, arg2: T2, arg3: T3, arg4: T4) => R, arg1: T1, arg2: T2): (arg3: T3, arg4: T4) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * The partial.placeholder value, which defaults to a `symbol`, may be used as a placeholder for partially applied arguments.
 *
 * Note: This method doesn't set the `length` property of partially applied functions.
 *
 * @template T1 The type of the first argument.
 * @template T2 The type of the second argument.
 * @template T3 The type of the third argument.
 * @template T4 The type of the fourth argument.
 * @template R The return type of the function.
 * @param {function(arg1: T1, arg2: T2, arg3: T3, arg4: T4): R} func The function to partially apply.
 * @param {T1} arg1 The first argument to apply.
 * @param {Placeholder} arg2 The placeholder for the second argument.
 * @param {T3} arg3 The third argument to apply.
 * @param {T4} arg4 The fourth argument to apply.
 * @returns {function(arg2: T2, arg4: T4): R} A new function that takes the second and fourth arguments and returns the result of the original function.
 *
 * @example
 * const greet = (greeting: string, name: string, punctuation: string) => `${greeting}, ${name}${punctuation}`;
 * const greetWithPlaceholder = partial(greet, 'Hello', partial.placeholder, '!');
 * console.log(greetWithPlaceholder('John')); // => 'Hello, John!'
 */
declare function partial<T1, T2, T3, T4, R>(func: (arg1: T1, arg2: T2, arg3: T3, arg4: T4) => R, arg1: T1, arg2: Placeholder, arg3: T3): (arg2: T2, arg4: T4) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * The partial.placeholder value, which defaults to a `symbol`, may be used as a placeholder for partially applied arguments.
 *
 * Note: This method doesn't set the `length` property of partially applied functions.
 *
 * @template T1 The type of the first argument.
 * @template T2 The type of the second argument.
 * @template T3 The type of the third argument.
 * @template T4 The type of the fourth argument.
 * @template R The return type of the function.
 * @param {function(arg1: T1, arg2: T2, arg3: T3, arg4: T4): R} func The function to partially apply.
 * @param {Placeholder} arg1 The placeholder for the first argument.
 * @param {T2} arg2 The second argument to apply.
 * @param {T3} arg3 The third argument to apply.
 * @param {T4} arg4 The fourth argument to apply.
 * @returns {function(arg1: T1, arg3: T3): R} A new function that takes the first and third arguments and returns the result of the original function.
 *
 * @example
 * const multiply = (x: number, y: number, z: number, w: number) => x * y * z * w;
 * const multiplyWithPlaceholder = partial(multiply, partial.placeholder, 2, 3);
 * console.log(multiplyWithPlaceholder(4)); // => 24
 */
declare function partial<T1, T2, T3, T4, R>(func: (arg1: T1, arg2: T2, arg3: T3, arg4: T4) => R, arg1: Placeholder, arg2: T2, arg3: T3): (arg1: T1, arg4: T4) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * The partial.placeholder value, which defaults to a `symbol`, may be used as a placeholder for partially applied arguments.
 *
 * Note: This method doesn't set the `length` property of partially applied functions.
 *
 * @template T1 The type of the first argument.
 * @template T2 The type of the second argument.
 * @template T3 The type of the third argument.
 * @template T4 The type of the fourth argument.
 * @template R The return type of the function.
 * @param {function(arg1: T1, arg2: T2, arg3: T3, arg4: T4): R} func The function to partially apply.
 * @param {Placeholder} arg1 The placeholder for the first argument.
 * @param {T2} arg2 The second argument to apply.
 * @param {Placeholder} arg3 The placeholder for the third argument.
 * @param {T4} arg4 The fourth argument to apply.
 * @returns {function(arg1: T1, arg3: T3): R} A new function that takes the first and third arguments and returns the result of the original function.
 */
declare function partial<T1, T2, T3, T4, R>(func: (arg1: T1, arg2: T2, arg3: T3, arg4: T4) => R, arg1: Placeholder, arg2: T2, arg3: Placeholder, arg4: T4): (arg1: T1, arg3: T3) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * The partial.placeholder value, which defaults to a `symbol`, may be used as a placeholder for partially applied arguments.
 *
 * Note: This method doesn't set the `length` property of partially applied functions.
 *
 * @template T1 The type of the first argument.
 * @template T2 The type of the second argument.
 * @template T3 The type of the third argument.
 * @template T4 The type of the fourth argument.
 * @template R The return type of the function.
 * @param {function(arg1: T1, arg2: T2, arg3: T3, arg4: T4): R} func The function to partially apply.
 * @param {Placeholder} arg1 The placeholder for the first argument.
 * @param {Placeholder} arg2 The placeholder for the second argument.
 * @param {T3} arg3 The third argument to apply.
 * @param {T4} arg4 The fourth argument to apply.
 * @returns {function(arg1: T1, arg2: T2): R} A new function that takes the first and second arguments and returns the result of the original function.
 *
 * @example
 * const multiply = (x: number, y: number, z: number, w: number) => x * y * z * w;
 * const multiplyWithPlaceholders = partial(multiply, partial.placeholder, partial.placeholder, 2, 3);
 * console.log(multiplyWithPlaceholders(4, 5)); // => 120
 */
declare function partial<T1, T2, T3, T4, R>(func: (arg1: T1, arg2: T2, arg3: T3, arg4: T4) => R, arg1: Placeholder, arg2: Placeholder, arg3: T3, arg4: T4): (arg1: T1, arg2: T2) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * The partial.placeholder value, which defaults to a `symbol`, may be used as a placeholder for partially applied arguments.
 *
 * Note: This method doesn't set the `length` property of partially applied functions.
 *
 * @template T1 The type of the first argument.
 * @template T2 The type of the second argument.
 * @template T3 The type of the third argument.
 * @template T4 The type of the fourth argument.
 * @template R The return type of the function.
 * @param {function(arg1: T1, arg2: T2, arg3: T3, arg4: T4): R} func The function to partially apply.
 * @param {T1} arg1 The first argument to apply.
 * @param {T2} arg2 The second argument to apply.
 * @param {T3} arg3 The third argument to apply.
 * @returns {function(arg4: T4): R} A new function that takes the fourth argument and returns the result of the original function.
 */
declare function partial<T1, T2, T3, T4, R>(func: (arg1: T1, arg2: T2, arg3: T3, arg4: T4) => R, arg1: T1, arg2: T2, arg3: T3): (arg4: T4) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * The partial.placeholder value, which defaults to a `symbol`, may be used as a placeholder for partially applied arguments.
 *
 * Note: This method doesn't set the `length` property of partially applied functions.
 *
 * @template T1 The type of the first argument.
 * @template T2 The type of the second argument.
 * @template T3 The type of the third argument.
 * @template T4 The type of the fourth argument.
 * @template R The return type of the function.
 * @param {function(arg1: T1, arg2: T2, arg3: T3, arg4: T4): R} func The function to partially apply.
 * @param {T1} arg1 The first argument to apply.
 * @param {T2} arg2 The second argument to apply.
 * @param {Placeholder} arg3 The placeholder for the third argument.
 * @param {T4} arg4 The fourth argument to apply.
 * @returns {function(arg3: T3): R} A new function that takes the third argument and returns the result of the original function.
 */
declare function partial<T1, T2, T3, T4, R>(func: (arg1: T1, arg2: T2, arg3: T3, arg4: T4) => R, arg1: T1, arg2: T2, arg3: Placeholder, arg4: T4): (arg3: T3) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * The partial.placeholder value, which defaults to a `symbol`, may be used as a placeholder for partially applied arguments.
 *
 * Note: This method doesn't set the `length` property of partially applied functions.
 *
 * @template T1 The type of the first argument.
 * @template T2 The type of the second argument.
 * @template T3 The type of the third argument.
 * @template T4 The type of the fourth argument.
 * @template R The return type of the function.
 * @param {function(arg1: T1, arg2: T2, arg3: T3, arg4: T4): R} func The function to partially apply.
 * @param {T1} arg1 The first argument to apply.
 * @param {Placeholder} arg2 The placeholder for the second argument.
 * @param {T3} arg3 The third argument to apply.
 * @param {T4} arg4 The fourth argument to apply.
 * @returns {function(arg2: T2): R} A new function that takes the second argument and returns the result of the original function.
 */
declare function partial<T1, T2, T3, T4, R>(func: (arg1: T1, arg2: T2, arg3: T3, arg4: T4) => R, arg1: T1, arg2: Placeholder, arg3: T3, arg4: T4): (arg2: T2) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * The partial.placeholder value, which defaults to a `symbol`, may be used as a placeholder for partially applied arguments.
 *
 * Note: This method doesn't set the `length` property of partially applied functions.
 *
 * @template T1 The type of the first argument.
 * @template T2 The type of the second argument.
 * @template T3 The type of the third argument.
 * @template T4 The type of the fourth argument.
 * @template R The return type of the function.
 * @param {function(arg1: T1, arg2: T2, arg3: T3, arg4: T4): R} func The function to partially apply.
 * @param {Placeholder} arg1 The placeholder for the first argument.
 * @param {T2} arg2 The second argument to apply.
 * @param {T3} arg3 The third argument to apply.
 * @param {T4} arg4 The fourth argument to apply.
 * @returns {function(arg1: T1): R} A new function that takes the first argument and returns the result of the original function.
 */
declare function partial<T1, T2, T3, T4, R>(func: (arg1: T1, arg2: T2, arg3: T3, arg4: T4) => R, arg1: Placeholder, arg2: T2, arg3: T3, arg4: T4): (arg1: T1) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * The partial.placeholder value, which defaults to a `symbol`, may be used as a placeholder for partially applied arguments.
 *
 * Note: This method doesn't set the `length` property of partially applied functions.
 *
 * @template TS The types of the arguments.
 * @template R The return type of the function.
 * @param {function(...args: TS): R} func The function to partially apply.
 * @returns {function(...args: TS): R} A new function that takes the same arguments as the original function.
 *
 * @example
 * const add = (...numbers: number[]) => numbers.reduce((sum, n) => sum + n, 0);
 * const addFive = partial(add, 5);
 * console.log(addFive(1, 2, 3)); // => 11
 */
declare function partial<TS extends any[], R>(func: (...args: TS) => R): (...args: TS) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * The partial.placeholder value, which defaults to a `symbol`, may be used as a placeholder for partially applied arguments.
 *
 * Note: This method doesn't set the `length` property of partially applied functions.
 *
 * @template TS The types of the arguments.
 * @template T1 The type of the first argument.
 * @template R The return type of the function.
 * @param {function(arg1: T1, ...args: TS): R} func The function to partially apply.
 * @param {T1} arg1 The first argument to apply.
 * @returns {function(...args: TS): R} A new function that takes the remaining arguments and returns the result of the original function.
 *
 * @example
 * const greet = (greeting: string, ...names: string[]) => `${greeting}, ${names.join(', ')}!`;
 * const greetHello = partial(greet, 'Hello');
 * console.log(greetHello('Alice', 'Bob')); // => 'Hello, Alice, Bob!'
 */
declare function partial<TS extends any[], T1, R>(func: (arg1: T1, ...args: TS) => R, arg1: T1): (...args: TS) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * The partial.placeholder value, which defaults to a `symbol`, may be used as a placeholder for partially applied arguments.
 *
 * Note: This method doesn't set the `length` property of partially applied functions.
 *
 * @template TS The types of the arguments.
 * @template T1 The type of the first argument.
 * @template T2 The type of the second argument.
 * @template R The return type of the function.
 * @param {function(arg1: T1, arg2: T2, ...args: TS): R} func The function to partially apply.
 * @param {T1} arg1 The first argument to apply.
 * @param {T2} arg2 The second argument to apply.
 * @returns {function(...args: TS): R} A new function that takes the remaining arguments and returns the result of the original function.
 *
 * @example
 * const greet = (greeting: string, name: string, punctuation: string) => `${greeting}, ${name}${punctuation}`;
 * const greetWithHello = partial(greet, 'Hello', '!');
 * console.log(greetWithHello('John')); // => 'Hello, John!'
 */
declare function partial<TS extends any[], T1, T2, R>(func: (arg1: T1, arg2: T2, ...args: TS) => R, t1: T1, arg2: T2): (...args: TS) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * The partial.placeholder value, which defaults to a `symbol`, may be used as a placeholder for partially applied arguments.
 *
 * Note: This method doesn't set the `length` property of partially applied functions.
 *
 * @template TS The types of the arguments.
 * @template T1 The type of the first argument.
 * @template T2 The type of the second argument.
 * @template T3 The type of the third argument.
 * @template R The return type of the function.
 * @param {function(t1: T1, arg2: T2, arg3: T3, ...args: TS): R} func The function to partially apply.
 * @param {T1} t1 The first argument to apply.
 * @param {T2} arg2 The second argument to apply.
 * @param {T3} arg3 The third argument to apply.
 * @returns {function(...args: TS): R} A new function that takes the remaining arguments and returns the result of the original function.
 *
 * @example
 * const greet = (greeting: string, name: string, punctuation: string) => `${greeting}, ${name}${punctuation}`;
 * const greetWithHello = partial(greet, 'Hello', 'John', '!');
 * console.log(greetWithHello()); // => 'Hello, John!'
 */
declare function partial<TS extends any[], T1, T2, T3, R>(func: (t1: T1, arg2: T2, arg3: T3, ...args: TS) => R, t1: T1, arg2: T2, arg3: T3): (...args: TS) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * The partial.placeholder value, which defaults to a `symbol`, may be used as a placeholder for partially applied arguments.
 *
 * Note: This method doesn't set the `length` property of partially applied functions.
 *
 * @template TS The types of the arguments.
 * @template T1 The type of the first argument.
 * @template T2 The type of the second argument.
 * @template T3 The type of the third argument.
 * @template T4 The type of the fourth argument.
 * @template R The return type of the function.
 * @param {function(t1: T1, arg2: T2, arg3: T3, arg4: T4, ...args: TS): R} func The function to partially apply.
 * @param {T1} t1 The first argument to apply.
 * @param {T2} arg2 The second argument to apply.
 * @param {T3} arg3 The third argument to apply.
 * @param {T4} arg4 The fourth argument to apply.
 * @returns {function(...args: TS): R} A new function that takes the remaining arguments and returns the result of the original function.
 *
 * @example
 * const greet = (greeting: string, name: string, punctuation: string) => `${greeting}, ${name}${punctuation}`;
 * const greetWithHello = partial(greet, 'Hello', 'John', '!');
 * console.log(greetWithHello()); // => 'Hello, John!'
 */
declare function partial<TS extends any[], T1, T2, T3, T4, R>(func: (t1: T1, arg2: T2, arg3: T3, arg4: T4, ...args: TS) => R, t1: T1, arg2: T2, arg3: T3, arg4: T4): (...args: TS) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * The partial.placeholder value, which defaults to a `symbol`, may be used as a placeholder for partially applied arguments.
 *
 * Note: This method doesn't set the `length` property of partially applied functions.
 *
 * @template F The type of the function to partially apply.
 * @param {F} func The function to partially apply.
 * @param {...any[]} partialArgs The arguments to be partially applied.
 * @returns {function(...args: any[]): ReturnType<F>} A new function that takes the remaining arguments and returns the result of the original function.
 *
 * @example
 * const add = (...numbers: number[]) => numbers.reduce((sum, n) => sum + n, 0);
 * const addFive = partial(add, 5);
 * console.log(addFive(1, 2, 3)); // => 11
 */
declare function partial<F extends (...args: any[]) => any>(func: F, ...partialArgs: any[]): (...args: any[]) => ReturnType<F>;
declare namespace partial {
    var placeholder: typeof placeholderSymbol;
}
declare const placeholderSymbol: unique symbol;
type Placeholder = typeof placeholderSymbol;

export { partial };
