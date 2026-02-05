import { Toolkit } from '../toolkit.js';

type __ = Placeholder | Toolkit;
/**
 * Creates a function that invokes the provided function with no arguments.
 *
 * @template R The return type of the function
 * @param {() => R} func The function to partially apply
 * @returns {() => R} Returns the new partially applied function
 *
 * @example
 * const greet = () => 'Hello!';
 * const sayHello = partialRight(greet);
 * sayHello(); // => 'Hello!'
 */
declare function partialRight<R>(func: () => R): () => R;
/**
 * Creates a function that invokes the provided function with one argument.
 *
 * @template T The type of the argument
 * @template R The return type of the function
 * @param {(t1: T) => R} func The function to partially apply
 * @returns {(t1: T) => R} Returns the new partially applied function
 *
 * @example
 * const greet = (name: string) => `Hello ${name}!`;
 * const greetPerson = partialRight(greet);
 * greetPerson('Fred'); // => 'Hello Fred!'
 */
declare function partialRight<T, R>(func: (t1: T) => R): (t1: T) => R;
/**
 * Creates a function that invokes the provided function with one argument pre-filled.
 *
 * @template T The type of the argument
 * @template R The return type of the function
 * @param {(t1: T) => R} func The function to partially apply
 * @param {T} arg1 The argument to pre-fill
 * @returns {() => R} Returns the new partially applied function
 *
 * @example
 * const greet = (name: string) => `Hello ${name}!`;
 * const greetFred = partialRight(greet, 'Fred');
 * greetFred(); // => 'Hello Fred!'
 */
declare function partialRight<T, R>(func: (t1: T) => R, arg1: T): () => R;
/**
 * Creates a function that invokes the provided function with two arguments.
 *
 * @template T1 The type of the first argument
 * @template T2 The type of the second argument
 * @template R The return type of the function
 * @param {(t1: T1, t2: T2) => R} func The function to partially apply
 * @returns {(t1: T1, t2: T2) => R} Returns the new partially applied function
 *
 * @example
 * const greet = (greeting: string, name: string) => `${greeting} ${name}!`;
 * const greetWithParams = partialRight(greet);
 * greetWithParams('Hi', 'Fred'); // => 'Hi Fred!'
 */
declare function partialRight<T1, T2, R>(func: (t1: T1, t2: T2) => R): (t1: T1, t2: T2) => R;
/**
 * Creates a function that invokes the provided function with one argument pre-filled and a placeholder.
 *
 * @template T1 The type of the first argument
 * @template T2 The type of the second argument
 * @template R The return type of the function
 * @param {(t1: T1, t2: T2) => R} func The function to partially apply
 * @param {T1} arg1 The argument to pre-fill
 * @param {__} plc2 The placeholder for the second argument
 * @returns {(t2: T2) => R} Returns the new partially applied function
 *
 * @example
 * const greet = (greeting: string, name: string) => `${greeting} ${name}!`;
 * const hiWithName = partialRight(greet, 'Hi', partialRight.placeholder);
 * hiWithName('Fred'); // => 'Hi Fred!'
 */
declare function partialRight<T1, T2, R>(func: (t1: T1, t2: T2) => R, arg1: T1, plc2: __): (t2: T2) => R;
/**
 * Creates a function that invokes the provided function with the second argument pre-filled.
 *
 * @template T1 The type of the first argument
 * @template T2 The type of the second argument
 * @template R The return type of the function
 * @param {(t1: T1, t2: T2) => R} func The function to partially apply
 * @param {T2} arg2 The argument to pre-fill
 * @returns {(t1: T1) => R} Returns the new partially applied function
 *
 * @example
 * const greet = (greeting: string, name: string) => `${greeting} ${name}!`;
 * const greetFred = partialRight(greet, 'Fred');
 * greetFred('Hi'); // => 'Hi Fred!'
 */
declare function partialRight<T1, T2, R>(func: (t1: T1, t2: T2) => R, arg2: T2): (t1: T1) => R;
/**
 * Creates a function that invokes the provided function with both arguments pre-filled.
 *
 * @template T1 The type of the first argument
 * @template T2 The type of the second argument
 * @template R The return type of the function
 * @param {(t1: T1, t2: T2) => R} func The function to partially apply
 * @param {T1} arg1 The first argument to pre-fill
 * @param {T2} arg2 The second argument to pre-fill
 * @returns {() => R} Returns the new partially applied function
 *
 * @example
 * const greet = (greeting: string, name: string) => `${greeting} ${name}!`;
 * const sayHiToFred = partialRight(greet, 'Hi', 'Fred');
 * sayHiToFred(); // => 'Hi Fred!'
 */
declare function partialRight<T1, T2, R>(func: (t1: T1, t2: T2) => R, arg1: T1, arg2: T2): () => R;
/**
 * Creates a function that invokes the provided function with no pre-filled arguments.
 *
 * @template T1 The type of the first argument
 * @template T2 The type of the second argument
 * @template T3 The type of the third argument
 * @template R The return type of the function
 * @param {(t1: T1, t2: T2, t3: T3) => R} func The function to partially apply
 * @returns {(t1: T1, t2: T2, t3: T3) => R} Returns the new partially applied function
 *
 * @example
 * const greet = (greeting: string, name: string, punctuation: string) => `${greeting} ${name}${punctuation}`;
 * const greetWithArgs = partialRight(greet);
 * greetWithArgs('Hi', 'Fred', '!'); // => 'Hi Fred!'
 */
declare function partialRight<T1, T2, T3, R>(func: (t1: T1, t2: T2, t3: T3) => R): (t1: T1, t2: T2, t3: T3) => R;
/**
 * Creates a function that invokes the provided function with the first argument pre-filled and placeholders for the rest.
 *
 * @template T1 The type of the first argument
 * @template T2 The type of the second argument
 * @template T3 The type of the third argument
 * @template R The return type of the function
 * @param {(t1: T1, t2: T2, t3: T3) => R} func The function to partially apply
 * @param {T1} arg1 The first argument to pre-fill
 * @param {__} plc2 The placeholder for the second argument
 * @param {__} plc3 The placeholder for the third argument
 * @returns {(t2: T2, t3: T3) => R} Returns the new partially applied function
 *
 * @example
 * const greet = (greeting: string, name: string, punctuation: string) => `${greeting} ${name}${punctuation}`;
 * const hiWithNameAndPunc = partialRight(greet, 'Hi', partialRight.placeholder, partialRight.placeholder);
 * hiWithNameAndPunc('Fred', '!'); // => 'Hi Fred!'
 */
declare function partialRight<T1, T2, T3, R>(func: (t1: T1, t2: T2, t3: T3) => R, arg1: T1, plc2: __, plc3: __): (t2: T2, t3: T3) => R;
/**
 * Creates a function that invokes the provided function with the second argument pre-filled and a placeholder for the third.
 *
 * @template T1 The type of the first argument
 * @template T2 The type of the second argument
 * @template T3 The type of the third argument
 * @template R The return type of the function
 * @param {(t1: T1, t2: T2, t3: T3) => R} func The function to partially apply
 * @param {T2} arg2 The second argument to pre-fill
 * @param {__} plc3 The placeholder for the third argument
 * @returns {(t1: T1, t3: T3) => R} Returns the new partially applied function
 *
 * @example
 * const greet = (greeting: string, name: string, punctuation: string) => `${greeting} ${name}${punctuation}`;
 * const greetFredWithPunc = partialRight(greet, 'Fred', partialRight.placeholder);
 * greetFredWithPunc('Hi', '!'); // => 'Hi Fred!'
 */
declare function partialRight<T1, T2, T3, R>(func: (t1: T1, t2: T2, t3: T3) => R, arg2: T2, plc3: __): (t1: T1, t3: T3) => R;
/**
 * Creates a function that invokes the provided function with the first two arguments pre-filled and a placeholder for the third.
 *
 * @template T1 The type of the first argument
 * @template T2 The type of the second argument
 * @template T3 The type of the third argument
 * @template R The return type of the function
 * @param {(t1: T1, t2: T2, t3: T3) => R} func The function to partially apply
 * @param {T1} arg1 The first argument to pre-fill
 * @param {T2} arg2 The second argument to pre-fill
 * @param {__} plc3 The placeholder for the third argument
 * @returns {(t3: T3) => R} Returns the new partially applied function
 *
 * @example
 * const greet = (greeting: string, name: string, punctuation: string) => `${greeting} ${name}${punctuation}`;
 * const hiToFredWithPunc = partialRight(greet, 'Hi', 'Fred', partialRight.placeholder);
 * hiToFredWithPunc('!'); // => 'Hi Fred!'
 */
declare function partialRight<T1, T2, T3, R>(func: (t1: T1, t2: T2, t3: T3) => R, arg1: T1, arg2: T2, plc3: __): (t3: T3) => R;
/**
 * Creates a function that invokes the provided function with the third argument pre-filled.
 *
 * @template T1 The type of the first argument
 * @template T2 The type of the second argument
 * @template T3 The type of the third argument
 * @template R The return type of the function
 * @param {(t1: T1, t2: T2, t3: T3) => R} func The function to partially apply
 * @param {T3} arg3 The third argument to pre-fill
 * @returns {(t1: T1, t2: T2) => R} Returns the new partially applied function
 *
 * @example
 * const greet = (greeting: string, name: string, punctuation: string) => `${greeting} ${name}${punctuation}`;
 * const greetWithExclamation = partialRight(greet, '!');
 * greetWithExclamation('Hi', 'Fred'); // => 'Hi Fred!'
 */
declare function partialRight<T1, T2, T3, R>(func: (t1: T1, t2: T2, t3: T3) => R, arg3: T3): (t1: T1, t2: T2) => R;
/**
 * Creates a function that invokes the provided function with the first and third arguments pre-filled.
 *
 * @template T1 The type of the first argument
 * @template T2 The type of the second argument
 * @template T3 The type of the third argument
 * @template R The return type of the function
 * @param {(t1: T1, t2: T2, t3: T3) => R} func The function to partially apply
 * @param {T1} arg1 The first argument to pre-fill
 * @param {__} plc2 The placeholder for the second argument
 * @param {T3} arg3 The third argument to pre-fill
 * @returns {(t2: T2) => R} Returns the new partially applied function
 *
 * @example
 * const greet = (greeting: string, name: string, punctuation: string) => `${greeting} ${name}${punctuation}`;
 * const hiWithNameAndExclamation = partialRight(greet, 'Hi', partialRight.placeholder, '!');
 * hiWithNameAndExclamation('Fred'); // => 'Hi Fred!'
 */
declare function partialRight<T1, T2, T3, R>(func: (t1: T1, t2: T2, t3: T3) => R, arg1: T1, plc2: __, arg3: T3): (t2: T2) => R;
/**
 * Creates a function that invokes the provided function with the second and third arguments pre-filled.
 *
 * @template T1 The type of the first argument
 * @template T2 The type of the second argument
 * @template T3 The type of the third argument
 * @template R The return type of the function
 * @param {(t1: T1, t2: T2, t3: T3) => R} func The function to partially apply
 * @param {T2} arg2 The second argument to pre-fill
 * @param {T3} arg3 The third argument to pre-fill
 * @returns {(t1: T1) => R} Returns the new partially applied function
 *
 * @example
 * const greet = (greeting: string, name: string, punctuation: string) => `${greeting} ${name}${punctuation}`;
 * const greetFredWithExclamation = partialRight(greet, 'Fred', '!');
 * greetFredWithExclamation('Hi'); // => 'Hi Fred!'
 */
declare function partialRight<T1, T2, T3, R>(func: (t1: T1, t2: T2, t3: T3) => R, arg2: T2, arg3: T3): (t1: T1) => R;
/**
 * Creates a function that invokes the provided function with all three arguments pre-filled.
 *
 * @template T1 The type of the first argument
 * @template T2 The type of the second argument
 * @template T3 The type of the third argument
 * @template R The return type of the function
 * @param {(t1: T1, t2: T2, t3: T3) => R} func The function to partially apply
 * @param {T1} arg1 The first argument to pre-fill
 * @param {T2} arg2 The second argument to pre-fill
 * @param {T3} arg3 The third argument to pre-fill
 * @returns {() => R} Returns the new partially applied function
 *
 * @example
 * const greet = (greeting: string, name: string, punctuation: string) => `${greeting} ${name}${punctuation}`;
 * const sayHiToFredWithExclamation = partialRight(greet, 'Hi', 'Fred', '!');
 * sayHiToFredWithExclamation(); // => 'Hi Fred!'
 */
declare function partialRight<T1, T2, T3, R>(func: (t1: T1, t2: T2, t3: T3) => R, arg1: T1, arg2: T2, arg3: T3): () => R;
/**
 * Creates a function that invokes the provided function with no pre-filled arguments.
 *
 * @template T1 The type of the first argument
 * @template T2 The type of the second argument
 * @template T3 The type of the third argument
 * @template T4 The type of the fourth argument
 * @template R The return type of the function
 * @param {(t1: T1, t2: T2, t3: T3, t4: T4) => R} func The function to partially apply
 * @returns {(t1: T1, t2: T2, t3: T3, t4: T4) => R} Returns the new partially applied function
 *
 * @example
 * const format = (greeting: string, name: string, time: string, punctuation: string) =>
 *   `${greeting} ${name}, it's ${time}${punctuation}`;
 * const formatWithArgs = partialRight(format);
 * formatWithArgs('Hi', 'Fred', 'morning', '!'); // => 'Hi Fred, it's morning!'
 */
declare function partialRight<T1, T2, T3, T4, R>(func: (t1: T1, t2: T2, t3: T3, t4: T4) => R): (t1: T1, t2: T2, t3: T3, t4: T4) => R;
/**
 * Creates a function that invokes the provided function with the first argument pre-filled and placeholders for the rest.
 *
 * @template T1 The type of the first argument
 * @template T2 The type of the second argument
 * @template T3 The type of the third argument
 * @template T4 The type of the fourth argument
 * @template R The return type of the function
 * @param {(t1: T1, t2: T2, t3: T3, t4: T4) => R} func The function to partially apply
 * @param {T1} arg1 The first argument to pre-fill
 * @param {__} plc2 The placeholder for the second argument
 * @param {__} plc3 The placeholder for the third argument
 * @param {__} plc4 The placeholder for the fourth argument
 * @returns {(t2: T2, t3: T3, t4: T4) => R} Returns the new partially applied function
 *
 * @example
 * const format = (greeting: string, name: string, time: string, punctuation: string) =>
 *   `${greeting} ${name}, it's ${time}${punctuation}`;
 * const hiWithRest = partialRight(format, 'Hi', partialRight.placeholder, partialRight.placeholder, partialRight.placeholder);
 * hiWithRest('Fred', 'morning', '!'); // => 'Hi Fred, it's morning!'
 */
declare function partialRight<T1, T2, T3, T4, R>(func: (t1: T1, t2: T2, t3: T3, t4: T4) => R, arg1: T1, plc2: __, plc3: __, plc4: __): (t2: T2, t3: T3, t4: T4) => R;
/**
 * Creates a function that invokes the provided function with the second argument pre-filled and placeholders for the rest.
 *
 * @template T1 The type of the first argument
 * @template T2 The type of the second argument
 * @template T3 The type of the third argument
 * @template T4 The type of the fourth argument
 * @template R The return type of the function
 * @param {(t1: T1, t2: T2, t3: T3, t4: T4) => R} func The function to partially apply
 * @param {T2} arg2 The second argument to pre-fill
 * @param {__} plc3 The placeholder for the third argument
 * @param {__} plc4 The placeholder for the fourth argument
 * @returns {(t1: T1, t3: T3, t4: T4) => R} Returns the new partially applied function
 *
 * @example
 * const format = (greeting: string, name: string, time: string, punctuation: string) =>
 *   `${greeting} ${name}, it's ${time}${punctuation}`;
 * const greetFredWithRest = partialRight(format, 'Fred', partialRight.placeholder, partialRight.placeholder);
 * greetFredWithRest('Hi', 'morning', '!'); // => 'Hi Fred, it's morning!'
 */
declare function partialRight<T1, T2, T3, T4, R>(func: (t1: T1, t2: T2, t3: T3, t4: T4) => R, arg2: T2, plc3: __, plc4: __): (t1: T1, t3: T3, t4: T4) => R;
/**
 * Creates a function that invokes the provided function with the first two arguments pre-filled and placeholders for the rest.
 *
 * @template T1 The type of the first argument
 * @template T2 The type of the second argument
 * @template T3 The type of the third argument
 * @template T4 The type of the fourth argument
 * @template R The return type of the function
 * @param {(t1: T1, t2: T2, t3: T3, t4: T4) => R} func The function to partially apply
 * @param {T1} arg1 The first argument to pre-fill
 * @param {T2} arg2 The second argument to pre-fill
 * @param {__} plc3 The placeholder for the third argument
 * @param {__} plc4 The placeholder for the fourth argument
 * @returns {(t3: T3, t4: T4) => R} Returns the new partially applied function
 *
 * @example
 * const format = (greeting: string, name: string, time: string, punctuation: string) =>
 *   `${greeting} ${name}, it's ${time}${punctuation}`;
 * const hiToFredWithRest = partialRight(format, 'Hi', 'Fred', partialRight.placeholder, partialRight.placeholder);
 * hiToFredWithRest('morning', '!'); // => 'Hi Fred, it's morning!'
 */
declare function partialRight<T1, T2, T3, T4, R>(func: (t1: T1, t2: T2, t3: T3, t4: T4) => R, arg1: T1, arg2: T2, plc3: __, plc4: __): (t3: T3, t4: T4) => R;
/**
 * Creates a function that invokes the provided function with the third argument pre-filled and a placeholder for the fourth.
 *
 * @template T1 The type of the first argument
 * @template T2 The type of the second argument
 * @template T3 The type of the third argument
 * @template T4 The type of the fourth argument
 * @template R The return type of the function
 * @param {(t1: T1, t2: T2, t3: T3, t4: T4) => R} func The function to partially apply
 * @param {T3} arg3 The third argument to pre-fill
 * @param {__} plc4 The placeholder for the fourth argument
 * @returns {(t1: T1, t2: T2, t4: T4) => R} Returns the new partially applied function
 *
 * @example
 * const format = (greeting: string, name: string, time: string, punctuation: string) =>
 *   `${greeting} ${name}, it's ${time}${punctuation}`;
 * const atMorningWithPunc = partialRight(format, 'morning', partialRight.placeholder);
 * atMorningWithPunc('Hi', 'Fred', '!'); // => 'Hi Fred, it's morning!'
 */
declare function partialRight<T1, T2, T3, T4, R>(func: (t1: T1, t2: T2, t3: T3, t4: T4) => R, arg3: T3, plc4: __): (t1: T1, t2: T2, t4: T4) => R;
/**
 * Creates a function that invokes the provided function with the first and third arguments pre-filled and a placeholder for the fourth.
 *
 * @template T1 The type of the first argument
 * @template T2 The type of the second argument
 * @template T3 The type of the third argument
 * @template T4 The type of the fourth argument
 * @template R The return type of the function
 * @param {(t1: T1, t2: T2, t3: T3, t4: T4) => R} func The function to partially apply
 * @param {T1} arg1 The first argument to pre-fill
 * @param {__} plc2 The placeholder for the second argument
 * @param {T3} arg3 The third argument to pre-fill
 * @param {__} plc4 The placeholder for the fourth argument
 * @returns {(t2: T2, t4: T4) => R} Returns the new partially applied function
 *
 * @example
 * const format = (greeting: string, name: string, time: string, punctuation: string) =>
 *   `${greeting} ${name}, it's ${time}${punctuation}`;
 * const hiAtMorningWithNameAndPunc = partialRight(format, 'Hi', partialRight.placeholder, 'morning', partialRight.placeholder);
 * hiAtMorningWithNameAndPunc('Fred', '!'); // => 'Hi Fred, it's morning!'
 */
declare function partialRight<T1, T2, T3, T4, R>(func: (t1: T1, t2: T2, t3: T3, t4: T4) => R, arg1: T1, plc2: __, arg3: T3, plc4: __): (t2: T2, t4: T4) => R;
/**
 * Creates a function that invokes the provided function with the second and third arguments pre-filled and a placeholder for the fourth.
 *
 * @template T1 The type of the first argument
 * @template T2 The type of the second argument
 * @template T3 The type of the third argument
 * @template T4 The type of the fourth argument
 * @template R The return type of the function
 * @param {(t1: T1, t2: T2, t3: T3, t4: T4) => R} func The function to partially apply
 * @param {T2} arg2 The second argument to pre-fill
 * @param {T3} arg3 The third argument to pre-fill
 * @param {__} plc4 The placeholder for the fourth argument
 * @returns {(t1: T1, t4: T4) => R} Returns the new partially applied function
 *
 * @example
 * const format = (greeting: string, name: string, time: string, punctuation: string) =>
 *   `${greeting} ${name}, it's ${time}${punctuation}`;
 * const greetFredAtMorningWithPunc = partialRight(format, 'Fred', 'morning', partialRight.placeholder);
 * greetFredAtMorningWithPunc('Hi', '!'); // => 'Hi Fred, it's morning!'
 */
declare function partialRight<T1, T2, T3, T4, R>(func: (t1: T1, t2: T2, t3: T3, t4: T4) => R, arg2: T2, arg3: T3, plc4: __): (t1: T1, t4: T4) => R;
/**
 * Creates a function that invokes the provided function with the first three arguments pre-filled and a placeholder for the fourth.
 *
 * @template T1 The type of the first argument
 * @template T2 The type of the second argument
 * @template T3 The type of the third argument
 * @template T4 The type of the fourth argument
 * @template R The return type of the function
 * @param {(t1: T1, t2: T2, t3: T3, t4: T4) => R} func The function to partially apply
 * @param {T1} arg1 The first argument to pre-fill
 * @param {T2} arg2 The second argument to pre-fill
 * @param {T3} arg3 The third argument to pre-fill
 * @param {__} plc4 The placeholder for the fourth argument
 * @returns {(t4: T4) => R} Returns the new partially applied function
 *
 * @example
 * const format = (greeting: string, name: string, time: string, punctuation: string) =>
 *   `${greeting} ${name}, it's ${time}${punctuation}`;
 * const hiToFredAtMorningWithPunc = partialRight(format, 'Hi', 'Fred', 'morning', partialRight.placeholder);
 * hiToFredAtMorningWithPunc('!'); // => 'Hi Fred, it's morning!'
 */
declare function partialRight<T1, T2, T3, T4, R>(func: (t1: T1, t2: T2, t3: T3, t4: T4) => R, arg1: T1, arg2: T2, arg3: T3, plc4: __): (t4: T4) => R;
/**
 * Creates a function that invokes the provided function with the fourth argument pre-filled.
 *
 * @template T1 The type of the first argument
 * @template T2 The type of the second argument
 * @template T3 The type of the third argument
 * @template T4 The type of the fourth argument
 * @template R The return type of the function
 * @param {(t1: T1, t2: T2, t3: T3, t4: T4) => R} func The function to partially apply
 * @param {T4} arg4 The fourth argument to pre-fill
 * @returns {(t1: T1, t2: T2, t3: T3) => R} Returns the new partially applied function
 *
 * @example
 * const format = (greeting: string, name: string, time: string, punctuation: string) =>
 *   `${greeting} ${name}, it's ${time}${punctuation}`;
 * const withExclamation = partialRight(format, '!');
 * withExclamation('Hi', 'Fred', 'morning'); // => 'Hi Fred, it's morning!'
 */
declare function partialRight<T1, T2, T3, T4, R>(func: (t1: T1, t2: T2, t3: T3, t4: T4) => R, arg4: T4): (t1: T1, t2: T2, t3: T3) => R;
/**
 * Creates a function that invokes the provided function with the first and fourth arguments pre-filled.
 *
 * @template T1 The type of the first argument
 * @template T2 The type of the second argument
 * @template T3 The type of the third argument
 * @template T4 The type of the fourth argument
 * @template R The return type of the function
 * @param {(t1: T1, t2: T2, t3: T3, t4: T4) => R} func The function to partially apply
 * @param {T1} arg1 The first argument to pre-fill
 * @param {__} plc2 The placeholder for the second argument
 * @param {__} plc3 The placeholder for the third argument
 * @param {T4} arg4 The fourth argument to pre-fill
 * @returns {(t2: T2, t3: T3) => R} Returns the new partially applied function
 *
 * @example
 * const format = (greeting: string, name: string, time: string, punctuation: string) =>
 *   `${greeting} ${name}, it's ${time}${punctuation}`;
 * const hiWithExclamation = partialRight(format, 'Hi', partialRight.placeholder, partialRight.placeholder, '!');
 * hiWithExclamation('Fred', 'morning'); // => 'Hi Fred, it's morning!'
 */
declare function partialRight<T1, T2, T3, T4, R>(func: (t1: T1, t2: T2, t3: T3, t4: T4) => R, arg1: T1, plc2: __, plc3: __, arg4: T4): (t2: T2, t3: T3) => R;
/**
 * Creates a function that invokes the provided function with the second and fourth arguments pre-filled and a placeholder for the third.
 *
 * @template T1 The type of the first argument
 * @template T2 The type of the second argument
 * @template T3 The type of the third argument
 * @template T4 The type of the fourth argument
 * @template R The return type of the function
 * @param {(t1: T1, t2: T2, t3: T3, t4: T4) => R} func The function to partially apply
 * @param {T2} arg2 The second argument to pre-fill
 * @param {__} plc3 The placeholder for the third argument
 * @param {T4} arg4 The fourth argument to pre-fill
 * @returns {(t1: T1, t3: T3) => R} Returns the new partially applied function
 *
 * @example
 * const format = (greeting: string, name: string, time: string, punctuation: string) =>
 *   `${greeting} ${name}, it's ${time}${punctuation}`;
 * const greetFredWithTime = partialRight(format, 'Fred', partialRight.placeholder, '!');
 * greetFredWithTime('Hi', 'morning'); // => 'Hi Fred, it's morning!'
 */
declare function partialRight<T1, T2, T3, T4, R>(func: (t1: T1, t2: T2, t3: T3, t4: T4) => R, arg2: T2, plc3: __, arg4: T4): (t1: T1, t3: T3) => R;
/**
 * Creates a function that invokes the provided function with the first, second and fourth arguments pre-filled and a placeholder for the third.
 *
 * @template T1 The type of the first argument
 * @template T2 The type of the second argument
 * @template T3 The type of the third argument
 * @template T4 The type of the fourth argument
 * @template R The return type of the function
 * @param {(t1: T1, t2: T2, t3: T3, t4: T4) => R} func The function to partially apply
 * @param {T1} arg1 The first argument to pre-fill
 * @param {T2} arg2 The second argument to pre-fill
 * @param {__} plc3 The placeholder for the third argument
 * @param {T4} arg4 The fourth argument to pre-fill
 * @returns {(t3: T3) => R} Returns the new partially applied function
 *
 * @example
 * const format = (greeting: string, name: string, time: string, punctuation: string) =>
 *   `${greeting} ${name}, it's ${time}${punctuation}`;
 * const hiToFredWithTime = partialRight(format, 'Hi', 'Fred', partialRight.placeholder, '!');
 * hiToFredWithTime('morning'); // => 'Hi Fred, it's morning!'
 */
declare function partialRight<T1, T2, T3, T4, R>(func: (t1: T1, t2: T2, t3: T3, t4: T4) => R, arg1: T1, arg2: T2, plc3: __, arg4: T4): (t3: T3) => R;
/**
 * Creates a function that invokes the provided function with the third and fourth arguments pre-filled.
 *
 * @template T1 The type of the first argument
 * @template T2 The type of the second argument
 * @template T3 The type of the third argument
 * @template T4 The type of the fourth argument
 * @template R The return type of the function
 * @param {(t1: T1, t2: T2, t3: T3, t4: T4) => R} func The function to partially apply
 * @param {T3} arg3 The third argument to pre-fill
 * @param {T4} arg4 The fourth argument to pre-fill
 * @returns {(t1: T1, t2: T2) => R} Returns the new partially applied function
 *
 * @example
 * const format = (greeting: string, name: string, time: string, punctuation: string) =>
 *   `${greeting} ${name}, it's ${time}${punctuation}`;
 * const inMorningWithExclamation = partialRight(format, 'morning', '!');
 * inMorningWithExclamation('Hi', 'Fred'); // => 'Hi Fred, it's morning!'
 */
declare function partialRight<T1, T2, T3, T4, R>(func: (t1: T1, t2: T2, t3: T3, t4: T4) => R, arg3: T3, arg4: T4): (t1: T1, t2: T2) => R;
/**
 * Creates a function that invokes the provided function with the first, third and fourth arguments pre-filled.
 *
 * @template T1 The type of the first argument
 * @template T2 The type of the second argument
 * @template T3 The type of the third argument
 * @template T4 The type of the fourth argument
 * @template R The return type of the function
 * @param {(t1: T1, t2: T2, t3: T3, t4: T4) => R} func The function to partially apply
 * @param {T1} arg1 The first argument to pre-fill
 * @param {__} plc2 The placeholder for the second argument
 * @param {T3} arg3 The third argument to pre-fill
 * @param {T4} arg4 The fourth argument to pre-fill
 * @returns {(t2: T2) => R} Returns the new partially applied function
 *
 * @example
 * const format = (greeting: string, name: string, time: string, punctuation: string) =>
 *   `${greeting} ${name}, it's ${time}${punctuation}`;
 * const hiInMorningWithExclamation = partialRight(format, 'Hi', partialRight.placeholder, 'morning', '!');
 * hiInMorningWithExclamation('Fred'); // => 'Hi Fred, it's morning!'
 */
declare function partialRight<T1, T2, T3, T4, R>(func: (t1: T1, t2: T2, t3: T3, t4: T4) => R, arg1: T1, plc2: __, arg3: T3, arg4: T4): (t2: T2) => R;
/**
 * Creates a function that invokes the provided function with the second, third and fourth arguments pre-filled.
 *
 * @template T1 The type of the first argument
 * @template T2 The type of the second argument
 * @template T3 The type of the third argument
 * @template T4 The type of the fourth argument
 * @template R The return type of the function
 * @param {(t1: T1, t2: T2, t3: T3, t4: T4) => R} func The function to partially apply
 * @param {T2} arg2 The second argument to pre-fill
 * @param {T3} arg3 The third argument to pre-fill
 * @param {T4} arg4 The fourth argument to pre-fill
 * @returns {(t1: T1) => R} Returns the new partially applied function
 *
 * @example
 * const format = (greeting: string, name: string, time: string, punctuation: string) =>
 *   `${greeting} ${name}, it's ${time}${punctuation}`;
 * const greetFredInMorningWithExclamation = partialRight(format, 'Fred', 'morning', '!');
 * greetFredInMorningWithExclamation('Hi'); // => 'Hi Fred, it's morning!'
 */
declare function partialRight<T1, T2, T3, T4, R>(func: (t1: T1, t2: T2, t3: T3, t4: T4) => R, arg2: T2, arg3: T3, arg4: T4): (t1: T1) => R;
/**
 * Creates a function that invokes the provided function with all arguments pre-filled.
 *
 * @template T1 The type of the first argument
 * @template T2 The type of the second argument
 * @template T3 The type of the third argument
 * @template T4 The type of the fourth argument
 * @template R The return type of the function
 * @param {(t1: T1, t2: T2, t3: T3, t4: T4) => R} func The function to partially apply
 * @param {T1} arg1 The first argument to pre-fill
 * @param {T2} arg2 The second argument to pre-fill
 * @param {T3} arg3 The third argument to pre-fill
 * @param {T4} arg4 The fourth argument to pre-fill
 * @returns {() => R} Returns the new partially applied function
 *
 * @example
 * const format = (greeting: string, name: string, time: string, punctuation: string) =>
 *   `${greeting} ${name}, it's ${time}${punctuation}`;
 * const sayHiToFredInMorningWithExclamation = partialRight(format, 'Hi', 'Fred', 'morning', '!');
 * sayHiToFredInMorningWithExclamation(); // => 'Hi Fred, it's morning!'
 */
declare function partialRight<T1, T2, T3, T4, R>(func: (t1: T1, t2: T2, t3: T3, t4: T4) => R, arg1: T1, arg2: T2, arg3: T3, arg4: T4): () => R;
/**
 * Creates a function that invokes the provided function with partially applied arguments appended to the arguments it receives.
 * The partialRight.placeholder value can be used as a placeholder for partially applied arguments.
 *
 * @template F The type of the function to partially apply
 * @param {F} func The function to partially apply arguments to
 * @param {...any[]} args The arguments to be partially applied
 * @returns {(...args: any[]) => ReturnType<F>} Returns the new partially applied function
 *
 * @example
 * function greet(greeting: string, name: string) {
 *   return greeting + ' ' + name;
 * }
 *
 * const greetFred = partialRight(greet, 'Fred');
 * greetFred('Hi'); // => 'Hi Fred'
 *
 * // Using placeholders
 * const sayHelloTo = partialRight(greet, 'Hello', partialRight.placeholder);
 * sayHelloTo('Fred'); // => 'Hello Fred'
 */
declare function partialRight(func: (...args: any[]) => any, ...args: any[]): (...args: any[]) => any;
declare namespace partialRight {
    var placeholder: Placeholder;
}
type Placeholder = symbol | (((value: any) => any) & {
    partialRight: typeof partialRight;
});

export { partialRight };
