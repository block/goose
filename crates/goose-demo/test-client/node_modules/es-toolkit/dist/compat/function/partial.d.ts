import { Toolkit } from '../toolkit.js';

type __ = Placeholder | Toolkit;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * The partial.placeholder value, which defaults to a `symbol`, may be used as a placeholder for partially applied arguments.
 *
 * @example
 * const greet = (greeting: string, name: string) => `${greeting} ${name}`;
 * const sayHello = partial(greet, _, 'world');
 * console.log(sayHello('Hello')); // => 'Hello world'
 */
declare function partial<T1, T2, R>(func: (t1: T1, t2: T2) => R, plc1: __, arg2: T2): (t1: T1) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * @example
 * const calculate = (x: number, y: number, z: number) => x + y + z;
 * const addToY = partial(calculate, _, 2);
 * console.log(addToY(1, 3)); // => 6
 */
declare function partial<T1, T2, T3, R>(func: (t1: T1, t2: T2, t3: T3) => R, plc1: __, arg2: T2): (t1: T1, t3: T3) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * @example
 * const calculate = (x: number, y: number, z: number) => x + y + z;
 * const addZ = partial(calculate, _, _, 3);
 * console.log(addZ(1, 2)); // => 6
 */
declare function partial<T1, T2, T3, R>(func: (t1: T1, t2: T2, t3: T3) => R, plc1: __, plc2: __, arg3: T3): (t1: T1, t2: T2) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * @example
 * const calculate = (x: number, y: number, z: number) => x + y + z;
 * const withXandZ = partial(calculate, 1, _, 3);
 * console.log(withXandZ(2)); // => 6
 */
declare function partial<T1, T2, T3, R>(func: (t1: T1, t2: T2, t3: T3) => R, arg1: T1, plc2: __, arg3: T3): (t2: T2) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * @example
 * const calculate = (x: number, y: number, z: number) => x + y + z;
 * const withYandZ = partial(calculate, _, 2, 3);
 * console.log(withYandZ(1)); // => 6
 */
declare function partial<T1, T2, T3, R>(func: (t1: T1, t2: T2, t3: T3) => R, plc1: __, arg2: T2, arg3: T3): (t1: T1) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * @example
 * const format = (a: string, b: string, c: string, d: string) => `${a}-${b}-${c}-${d}`;
 * const withB = partial(format, _, 'b');
 * console.log(withB('a', 'c', 'd')); // => 'a-b-c-d'
 */
declare function partial<T1, T2, T3, T4, R>(func: (t1: T1, t2: T2, t3: T3, t4: T4) => R, plc1: __, arg2: T2): (t1: T1, t3: T3, t4: T4) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * @example
 * const format = (a: string, b: string, c: string, d: string) => `${a}-${b}-${c}-${d}`;
 * const withC = partial(format, _, _, 'c');
 * console.log(withC('a', 'b', 'd')); // => 'a-b-c-d'
 */
declare function partial<T1, T2, T3, T4, R>(func: (t1: T1, t2: T2, t3: T3, t4: T4) => R, plc1: __, plc2: __, arg3: T3): (t1: T1, t2: T2, t4: T4) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * @example
 * const format = (a: string, b: string, c: string, d: string) => `${a}-${b}-${c}-${d}`;
 * const withAandC = partial(format, 'a', _, 'c');
 * console.log(withAandC('b', 'd')); // => 'a-b-c-d'
 */
declare function partial<T1, T2, T3, T4, R>(func: (t1: T1, t2: T2, t3: T3, t4: T4) => R, arg1: T1, plc2: __, arg3: T3): (t2: T2, t4: T4) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * @example
 * const format = (a: string, b: string, c: string, d: string) => `${a}-${b}-${c}-${d}`;
 * const withBandC = partial(format, _, 'b', 'c');
 * console.log(withBandC('a', 'd')); // => 'a-b-c-d'
 */
declare function partial<T1, T2, T3, T4, R>(func: (t1: T1, t2: T2, t3: T3, t4: T4) => R, plc1: __, arg2: T2, arg3: T3): (t1: T1, t4: T4) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * @example
 * const format = (a: string, b: string, c: string, d: string) => `${a}-${b}-${c}-${d}`;
 * const withD = partial(format, _, _, _, 'd');
 * console.log(withD('a', 'b', 'c')); // => 'a-b-c-d'
 */
declare function partial<T1, T2, T3, T4, R>(func: (t1: T1, t2: T2, t3: T3, t4: T4) => R, plc1: __, plc2: __, plc3: __, arg4: T4): (t1: T1, t2: T2, t3: T3) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * @example
 * const format = (a: string, b: string, c: string, d: string) => `${a}-${b}-${c}-${d}`;
 * const withAandD = partial(format, 'a', _, _, 'd');
 * console.log(withAandD('b', 'c')); // => 'a-b-c-d'
 */
declare function partial<T1, T2, T3, T4, R>(func: (t1: T1, t2: T2, t3: T3, t4: T4) => R, arg1: T1, plc2: __, plc3: __, arg4: T4): (t2: T2, t3: T3) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * @example
 * const format = (a: string, b: string, c: string, d: string) => `${a}-${b}-${c}-${d}`;
 * const withBandD = partial(format, _, 'b', _, 'd');
 * console.log(withBandD('a', 'c')); // => 'a-b-c-d'
 */
declare function partial<T1, T2, T3, T4, R>(func: (t1: T1, t2: T2, t3: T3, t4: T4) => R, plc1: __, arg2: T2, plc3: __, arg4: T4): (t1: T1, t3: T3) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * @example
 * const format = (a: string, b: string, c: string, d: string) => `${a}-${b}-${c}-${d}`;
 * const withABandD = partial(format, 'a', 'b', _, 'd');
 * console.log(withABandD('c')); // => 'a-b-c-d'
 */
declare function partial<T1, T2, T3, T4, R>(func: (t1: T1, t2: T2, t3: T3, t4: T4) => R, arg1: T1, arg2: T2, plc3: __, arg4: T4): (t3: T3) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * @example
 * const format = (a: string, b: string, c: string, d: string) => `${a}-${b}-${c}-${d}`;
 * const withCandD = partial(format, _, _, 'c', 'd');
 * console.log(withCandD('a', 'b')); // => 'a-b-c-d'
 */
declare function partial<T1, T2, T3, T4, R>(func: (t1: T1, t2: T2, t3: T3, t4: T4) => R, plc1: __, plc2: __, arg3: T3, arg4: T4): (t1: T1, t2: T2) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * @example
 * const format = (a: string, b: string, c: string, d: string) => `${a}-${b}-${c}-${d}`;
 * const withACandD = partial(format, 'a', _, 'c', 'd');
 * console.log(withACandD('b')); // => 'a-b-c-d'
 */
declare function partial<T1, T2, T3, T4, R>(func: (t1: T1, t2: T2, t3: T3, t4: T4) => R, arg1: T1, plc2: __, arg3: T3, arg4: T4): (t2: T2) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * @example
 * const format = (a: string, b: string, c: string, d: string) => `${a}-${b}-${c}-${d}`;
 * const withBCandD = partial(format, _, 'b', 'c', 'd');
 * console.log(withBCandD('a')); // => 'a-b-c-d'
 */
declare function partial<T1, T2, T3, T4, R>(func: (t1: T1, t2: T2, t3: T3, t4: T4) => R, plc1: __, arg2: T2, arg3: T3, arg4: T4): (t1: T1) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * @example
 * const sum = (...numbers: number[]) => numbers.reduce((a, b) => a + b, 0);
 * const partialSum = partial(sum);
 * console.log(partialSum(1, 2, 3)); // => 6
 */
declare function partial<TS extends any[], R>(func: (...ts: TS) => R): (...ts: TS) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * @example
 * const log = (prefix: string, ...messages: string[]) => console.log(prefix, ...messages);
 * const debugLog = partial(log, '[DEBUG]');
 * debugLog('message 1', 'message 2'); // => '[DEBUG] message 1 message 2'
 */
declare function partial<TS extends any[], T1, R>(func: (t1: T1, ...ts: TS) => R, arg1: T1): (...ts: TS) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * @example
 * const format = (prefix: string, separator: string, ...messages: string[]) => `${prefix}${messages.join(separator)}`;
 * const logWithPrefix = partial(format, '[LOG]', ' - ');
 * console.log(logWithPrefix('msg1', 'msg2')); // => '[LOG]msg1 - msg2'
 */
declare function partial<TS extends any[], T1, T2, R>(func: (t1: T1, t2: T2, ...ts: TS) => R, t1: T1, t2: T2): (...ts: TS) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * @example
 * const format = (type: string, level: string, message: string, ...tags: string[]) =>
 *   `[${type}][${level}] ${message} ${tags.join(',')}`;
 * const errorLog = partial(format, 'ERROR', 'HIGH', 'Something went wrong');
 * console.log(errorLog('critical', 'urgent')); // => '[ERROR][HIGH] Something went wrong critical,urgent'
 */
declare function partial<TS extends any[], T1, T2, T3, R>(func: (t1: T1, t2: T2, t3: T3, ...ts: TS) => R, t1: T1, t2: T2, t3: T3): (...ts: TS) => R;
/**
 * Creates a function that invokes `func` with `partialArgs` prepended to the arguments it receives. This method is like `bind` except it does not alter the `this` binding.
 *
 * @example
 * const format = (a: string, b: string, c: string, d: string, ...rest: string[]) =>
 *   `${a}-${b}-${c}-${d}:${rest.join(',')}`;
 * const prefixedFormat = partial(format, 'a', 'b', 'c', 'd');
 * console.log(prefixedFormat('e', 'f')); // => 'a-b-c-d:e,f'
 */
declare function partial<TS extends any[], T1, T2, T3, T4, R>(func: (t1: T1, t2: T2, t3: T3, t4: T4, ...ts: TS) => R, t1: T1, t2: T2, t3: T3, t4: T4): (...ts: TS) => R;
declare namespace partial {
    var placeholder: Placeholder;
}
type Placeholder = symbol | (((value: any) => any) & {
    partial: typeof partial;
});

export { partial };
