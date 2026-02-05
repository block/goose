type __ = typeof curryRightPlaceholder;
interface RightCurriedFunction1<T1, R> {
    (): RightCurriedFunction1<T1, R>;
    (t1: T1): R;
}
interface RightCurriedFunction2<T1, T2, R> {
    (): RightCurriedFunction2<T1, T2, R>;
    (t2: T2): RightCurriedFunction1<T1, R>;
    (t1: T1, t2: __): RightCurriedFunction1<T2, R>;
    (t1: T1, t2: T2): R;
}
interface RightCurriedFunction3<T1, T2, T3, R> {
    (): RightCurriedFunction3<T1, T2, T3, R>;
    (t3: T3): RightCurriedFunction2<T1, T2, R>;
    (t2: T2, t3: __): RightCurriedFunction2<T1, T3, R>;
    (t2: T2, t3: T3): RightCurriedFunction1<T1, R>;
    (t1: T1, t2: __, t3: __): RightCurriedFunction2<T2, T3, R>;
    (t1: T1, t2: T2, t3: __): RightCurriedFunction1<T3, R>;
    (t1: T1, t2: __, t3: T3): RightCurriedFunction1<T2, R>;
    (t1: T1, t2: T2, t3: T3): R;
}
interface RightCurriedFunction4<T1, T2, T3, T4, R> {
    (): RightCurriedFunction4<T1, T2, T3, T4, R>;
    (t4: T4): RightCurriedFunction3<T1, T2, T3, R>;
    (t3: T3, t4: __): RightCurriedFunction3<T1, T2, T4, R>;
    (t3: T3, t4: T4): RightCurriedFunction2<T1, T2, R>;
    (t2: T2, t3: __, t4: __): RightCurriedFunction3<T1, T3, T4, R>;
    (t2: T2, t3: T3, t4: __): RightCurriedFunction2<T1, T4, R>;
    (t2: T2, t3: __, t4: T4): RightCurriedFunction2<T1, T3, R>;
    (t2: T2, t3: T3, t4: T4): RightCurriedFunction1<T1, R>;
    (t1: T1, t2: __, t3: __, t4: __): RightCurriedFunction3<T2, T3, T4, R>;
    (t1: T1, t2: T2, t3: __, t4: __): RightCurriedFunction2<T3, T4, R>;
    (t1: T1, t2: __, t3: T3, t4: __): RightCurriedFunction2<T2, T4, R>;
    (t1: T1, t2: __, t3: __, t4: T4): RightCurriedFunction2<T2, T3, R>;
    (t1: T1, t2: T2, t3: T3, t4: __): RightCurriedFunction1<T4, R>;
    (t1: T1, t2: T2, t3: __, t4: T4): RightCurriedFunction1<T3, R>;
    (t1: T1, t2: __, t3: T3, t4: T4): RightCurriedFunction1<T2, R>;
    (t1: T1, t2: T2, t3: T3, t4: T4): R;
}
interface RightCurriedFunction5<T1, T2, T3, T4, T5, R> {
    (): RightCurriedFunction5<T1, T2, T3, T4, T5, R>;
    (t5: T5): RightCurriedFunction4<T1, T2, T3, T4, R>;
    (t4: T4, t5: __): RightCurriedFunction4<T1, T2, T3, T5, R>;
    (t4: T4, t5: T5): RightCurriedFunction3<T1, T2, T3, R>;
    (t3: T3, t4: __, t5: __): RightCurriedFunction4<T1, T2, T4, T5, R>;
    (t3: T3, t4: T4, t5: __): RightCurriedFunction3<T1, T2, T5, R>;
    (t3: T3, t4: __, t5: T5): RightCurriedFunction3<T1, T2, T4, R>;
    (t3: T3, t4: T4, t5: T5): RightCurriedFunction2<T1, T2, R>;
    (t2: T2, t3: __, t4: __, t5: __): RightCurriedFunction4<T1, T3, T4, T5, R>;
    (t2: T2, t3: T3, t4: __, t5: __): RightCurriedFunction3<T1, T4, T5, R>;
    (t2: T2, t3: __, t4: T4, t5: __): RightCurriedFunction3<T1, T3, T5, R>;
    (t2: T2, t3: __, t4: __, t5: T5): RightCurriedFunction3<T1, T3, T4, R>;
    (t2: T2, t3: T3, t4: T4, t5: __): RightCurriedFunction2<T1, T5, R>;
    (t2: T2, t3: T3, t4: __, t5: T5): RightCurriedFunction2<T1, T4, R>;
    (t2: T2, t3: __, t4: T4, t5: T5): RightCurriedFunction2<T1, T3, R>;
    (t2: T2, t3: T3, t4: T4, t5: T5): RightCurriedFunction1<T1, R>;
    (t1: T1, t2: __, t3: __, t4: __, t5: __): RightCurriedFunction4<T2, T3, T4, T5, R>;
    (t1: T1, t2: T2, t3: __, t4: __, t5: __): RightCurriedFunction3<T3, T4, T5, R>;
    (t1: T1, t2: __, t3: T3, t4: __, t5: __): RightCurriedFunction3<T2, T4, T5, R>;
    (t1: T1, t2: __, t3: __, t4: T4, t5: __): RightCurriedFunction3<T2, T3, T5, R>;
    (t1: T1, t2: __, t3: __, t4: __, t5: T5): RightCurriedFunction3<T2, T3, T4, R>;
    (t1: T1, t2: T2, t3: T3, t4: __, t5: __): RightCurriedFunction2<T4, T5, R>;
    (t1: T1, t2: T2, t3: __, t4: T4, t5: __): RightCurriedFunction2<T3, T5, R>;
    (t1: T1, t2: T2, t3: __, t4: __, t5: T5): RightCurriedFunction2<T3, T4, R>;
    (t1: T1, t2: __, t3: T3, t4: T4, t5: __): RightCurriedFunction2<T2, T5, R>;
    (t1: T1, t2: __, t3: T3, t4: __, t5: T5): RightCurriedFunction2<T2, T4, R>;
    (t1: T1, t2: __, t3: __, t4: T4, t5: T5): RightCurriedFunction2<T2, T3, R>;
    (t1: T1, t2: T2, t3: T3, t4: T4, t5: __): RightCurriedFunction1<T5, R>;
    (t1: T1, t2: T2, t3: T3, t4: __, t5: T5): RightCurriedFunction1<T4, R>;
    (t1: T1, t2: T2, t3: __, t4: T4, t5: T5): RightCurriedFunction1<T3, R>;
    (t1: T1, t2: __, t3: T3, t4: T4, t5: T5): RightCurriedFunction1<T2, R>;
    (t1: T1, t2: T2, t3: T3, t4: T4, t5: T5): R;
}
declare function curryRight<T1, R>(func: (t1: T1) => R, arity?: number): RightCurriedFunction1<T1, R>;
declare function curryRight<T1, T2, R>(func: (t1: T1, t2: T2) => R, arity?: number): RightCurriedFunction2<T1, T2, R>;
declare function curryRight<T1, T2, T3, R>(func: (t1: T1, t2: T2, t3: T3) => R, arity?: number): RightCurriedFunction3<T1, T2, T3, R>;
declare function curryRight<T1, T2, T3, T4, R>(func: (t1: T1, t2: T2, t3: T3, t4: T4) => R, arity?: number): RightCurriedFunction4<T1, T2, T3, T4, R>;
declare function curryRight<T1, T2, T3, T4, T5, R>(func: (t1: T1, t2: T2, t3: T3, t4: T4, t5: T5) => R, arity?: number): RightCurriedFunction5<T1, T2, T3, T4, T5, R>;
declare function curryRight(func: (...args: any[]) => any, arity?: number): (...args: any[]) => any;
declare namespace curryRight {
    var placeholder: typeof curryRightPlaceholder;
}
declare const curryRightPlaceholder: unique symbol;

export { curryRight };
