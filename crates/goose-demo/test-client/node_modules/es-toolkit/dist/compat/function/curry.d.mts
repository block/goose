type __ = typeof curryPlaceholder;
interface CurriedFunction1<T1, R> {
    (): CurriedFunction1<T1, R>;
    (t1: T1): R;
}
interface CurriedFunction2<T1, T2, R> {
    (): CurriedFunction2<T1, T2, R>;
    (t1: T1): CurriedFunction1<T2, R>;
    (t1: __, t2: T2): CurriedFunction1<T1, R>;
    (t1: T1, t2: T2): R;
}
interface CurriedFunction3<T1, T2, T3, R> {
    (): CurriedFunction3<T1, T2, T3, R>;
    (t1: T1): CurriedFunction2<T2, T3, R>;
    (t1: __, t2: T2): CurriedFunction2<T1, T3, R>;
    (t1: T1, t2: T2): CurriedFunction1<T3, R>;
    (t1: __, t2: __, t3: T3): CurriedFunction2<T1, T2, R>;
    (t1: T1, t2: __, t3: T3): CurriedFunction1<T2, R>;
    (t1: __, t2: T2, t3: T3): CurriedFunction1<T1, R>;
    (t1: T1, t2: T2, t3: T3): R;
}
interface CurriedFunction4<T1, T2, T3, T4, R> {
    (): CurriedFunction4<T1, T2, T3, T4, R>;
    (t1: T1): CurriedFunction3<T2, T3, T4, R>;
    (t1: __, t2: T2): CurriedFunction3<T1, T3, T4, R>;
    (t1: T1, t2: T2): CurriedFunction2<T3, T4, R>;
    (t1: __, t2: __, t3: T3): CurriedFunction3<T1, T2, T4, R>;
    (t1: __, t2: __, t3: T3): CurriedFunction2<T2, T4, R>;
    (t1: __, t2: T2, t3: T3): CurriedFunction2<T1, T4, R>;
    (t1: T1, t2: T2, t3: T3): CurriedFunction1<T4, R>;
    (t1: __, t2: __, t3: __, t4: T4): CurriedFunction3<T1, T2, T3, R>;
    (t1: T1, t2: __, t3: __, t4: T4): CurriedFunction2<T2, T3, R>;
    (t1: __, t2: T2, t3: __, t4: T4): CurriedFunction2<T1, T3, R>;
    (t1: __, t2: __, t3: T3, t4: T4): CurriedFunction2<T1, T2, R>;
    (t1: T1, t2: T2, t3: __, t4: T4): CurriedFunction1<T3, R>;
    (t1: T1, t2: __, t3: T3, t4: T4): CurriedFunction1<T2, R>;
    (t1: __, t2: T2, t3: T3, t4: T4): CurriedFunction1<T1, R>;
    (t1: T1, t2: T2, t3: T3, t4: T4): R;
}
interface CurriedFunction5<T1, T2, T3, T4, T5, R> {
    (): CurriedFunction5<T1, T2, T3, T4, T5, R>;
    (t1: T1): CurriedFunction4<T2, T3, T4, T5, R>;
    (t1: __, t2: T2): CurriedFunction4<T1, T3, T4, T5, R>;
    (t1: T1, t2: T2): CurriedFunction3<T3, T4, T5, R>;
    (t1: __, t2: __, t3: T3): CurriedFunction4<T1, T2, T4, T5, R>;
    (t1: T1, t2: __, t3: T3): CurriedFunction3<T2, T4, T5, R>;
    (t1: __, t2: T2, t3: T3): CurriedFunction3<T1, T4, T5, R>;
    (t1: T1, t2: T2, t3: T3): CurriedFunction2<T4, T5, R>;
    (t1: __, t2: __, t3: __, t4: T4): CurriedFunction4<T1, T2, T3, T5, R>;
    (t1: T1, t2: __, t3: __, t4: T4): CurriedFunction3<T2, T3, T5, R>;
    (t1: __, t2: T2, t3: __, t4: T4): CurriedFunction3<T1, T3, T5, R>;
    (t1: __, t2: __, t3: T3, t4: T4): CurriedFunction3<T1, T2, T5, R>;
    (t1: T1, t2: T2, t3: __, t4: T4): CurriedFunction2<T3, T5, R>;
    (t1: T1, t2: __, t3: T3, t4: T4): CurriedFunction2<T2, T5, R>;
    (t1: __, t2: T2, t3: T3, t4: T4): CurriedFunction2<T1, T5, R>;
    (t1: T1, t2: T2, t3: T3, t4: T4): CurriedFunction1<T5, R>;
    (t1: __, t2: __, t3: __, t4: __, t5: T5): CurriedFunction4<T1, T2, T3, T4, R>;
    (t1: T1, t2: __, t3: __, t4: __, t5: T5): CurriedFunction3<T2, T3, T4, R>;
    (t1: __, t2: T2, t3: __, t4: __, t5: T5): CurriedFunction3<T1, T3, T4, R>;
    (t1: __, t2: __, t3: T3, t4: __, t5: T5): CurriedFunction3<T1, T2, T4, R>;
    (t1: __, t2: __, t3: __, t4: T4, t5: T5): CurriedFunction3<T1, T2, T3, R>;
    (t1: T1, t2: T2, t3: __, t4: __, t5: T5): CurriedFunction2<T3, T4, R>;
    (t1: T1, t2: __, t3: T3, t4: __, t5: T5): CurriedFunction2<T2, T4, R>;
    (t1: T1, t2: __, t3: __, t4: T4, t5: T5): CurriedFunction2<T2, T3, R>;
    (t1: __, t2: T2, t3: T3, t4: __, t5: T5): CurriedFunction2<T1, T4, R>;
    (t1: __, t2: T2, t3: __, t4: T4, t5: T5): CurriedFunction2<T1, T3, R>;
    (t1: __, t2: __, t3: T3, t4: T4, t5: T5): CurriedFunction2<T1, T2, R>;
    (t1: T1, t2: T2, t3: T3, t4: __, t5: T5): CurriedFunction1<T4, R>;
    (t1: T1, t2: T2, t3: __, t4: T4, t5: T5): CurriedFunction1<T3, R>;
    (t1: T1, t2: __, t3: T3, t4: T4, t5: T5): CurriedFunction1<T2, R>;
    (t1: __, t2: T2, t3: T3, t4: T4, t5: T5): CurriedFunction1<T1, R>;
    (t1: T1, t2: T2, t3: T3, t4: T4, t5: T5): R;
}
/**
 * Creates a curried function that accepts a single argument.
 * @param {(t1: T1) => R} func - The function to curry.
 * @param {number=func.length} arity - The arity of func.
 * @returns {CurriedFunction1<T1, R>} - Returns the new curried function.
 * @example
 * const greet = (name: string) => `Hello ${name}`;
 * const curriedGreet = curry(greet);
 * curriedGreet('John'); // => 'Hello John'
 */
declare function curry<T1, R>(func: (t1: T1) => R, arity?: number): CurriedFunction1<T1, R>;
/**
 * Creates a curried function that accepts two arguments.
 * @param {(t1: T1, t2: T2) => R} func - The function to curry.
 * @param {number=func.length} arity - The arity of func.
 * @returns {CurriedFunction2<T1, T2, R>} - Returns the new curried function.
 * @example
 * const add = (a: number, b: number) => a + b;
 * const curriedAdd = curry(add);
 * curriedAdd(1)(2); // => 3
 * curriedAdd(1, 2); // => 3
 */
declare function curry<T1, T2, R>(func: (t1: T1, t2: T2) => R, arity?: number): CurriedFunction2<T1, T2, R>;
/**
 * Creates a curried function that accepts three arguments.
 * @param {(t1: T1, t2: T2, t3: T3) => R} func - The function to curry.
 * @param {number=func.length} arity - The arity of func.
 * @returns {CurriedFunction3<T1, T2, T3, R>} - Returns the new curried function.
 * @example
 * const volume = (l: number, w: number, h: number) => l * w * h;
 * const curriedVolume = curry(volume);
 * curriedVolume(2)(3)(4); // => 24
 * curriedVolume(2, 3)(4); // => 24
 * curriedVolume(2, 3, 4); // => 24
 */
declare function curry<T1, T2, T3, R>(func: (t1: T1, t2: T2, t3: T3) => R, arity?: number): CurriedFunction3<T1, T2, T3, R>;
/**
 * Creates a curried function that accepts four arguments.
 * @param {(t1: T1, t2: T2, t3: T3, t4: T4) => R} func - The function to curry.
 * @param {number=func.length} arity - The arity of func.
 * @returns {CurriedFunction4<T1, T2, T3, T4, R>} - Returns the new curried function.
 * @example
 * const fn = (a: number, b: number, c: number, d: number) => a + b + c + d;
 * const curriedFn = curry(fn);
 * curriedFn(1)(2)(3)(4); // => 10
 * curriedFn(1, 2)(3, 4); // => 10
 * curriedFn(1, 2, 3, 4); // => 10
 */
declare function curry<T1, T2, T3, T4, R>(func: (t1: T1, t2: T2, t3: T3, t4: T4) => R, arity?: number): CurriedFunction4<T1, T2, T3, T4, R>;
/**
 * Creates a curried function that accepts five arguments.
 * @param {(t1: T1, t2: T2, t3: T3, t4: T4, t5: T5) => R} func - The function to curry.
 * @param {number=func.length} arity - The arity of func.
 * @returns {CurriedFunction5<T1, T2, T3, T4, T5, R>} - Returns the new curried function.
 * @example
 * const fn = (a: number, b: number, c: number, d: number, e: number) => a + b + c + d + e;
 * const curriedFn = curry(fn);
 * curriedFn(1)(2)(3)(4)(5); // => 15
 * curriedFn(1, 2)(3, 4)(5); // => 15
 * curriedFn(1, 2, 3, 4, 5); // => 15
 */
declare function curry<T1, T2, T3, T4, T5, R>(func: (t1: T1, t2: T2, t3: T3, t4: T4, t5: T5) => R, arity?: number): CurriedFunction5<T1, T2, T3, T4, T5, R>;
/**
 * Creates a curried function that accepts any number of arguments.
 * @param {(...args: any[]) => any} func - The function to curry.
 * @param {number=func.length} arity - The arity of func.
 * @returns {(...args: any[]) => any} - Returns the new curried function.
 * @example
 * const sum = (...args: number[]) => args.reduce((a, b) => a + b, 0);
 * const curriedSum = curry(sum);
 * curriedSum(1, 2, 3); // => 6
 * curriedSum(1)(2, 3); // => 6
 * curriedSum(1)(2)(3); // => 6
 */
declare function curry(func: (...args: any[]) => any, arity?: number): (...args: any[]) => any;
declare namespace curry {
    var placeholder: typeof curryPlaceholder;
}
declare const curryPlaceholder: unique symbol;

export { curry };
