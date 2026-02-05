type ValueIteratorTypeGuard<T, S extends T> = (value: T) => value is S;

export type { ValueIteratorTypeGuard };
