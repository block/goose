type ListIteratorTypeGuard<T, S extends T> = (value: T, index: number, collection: ArrayLike<T>) => value is S;

export type { ListIteratorTypeGuard };
