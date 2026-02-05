type TupleIterator<T extends readonly unknown[], TResult> = (value: T[number], index: T extends `${infer N extends number}` ? N : never, collection: T) => TResult;

export type { TupleIterator };
