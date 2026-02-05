interface RecursiveArray<T> extends Array<T | RecursiveArray<T>> {
}

export type { RecursiveArray };
